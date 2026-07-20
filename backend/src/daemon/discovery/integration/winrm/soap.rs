//! WS-Management SOAP envelope construction/parsing, scoped to exactly the
//! five operations a WinRM "cmd" shell session needs: Create, Command,
//! Receive, Signal, Delete. Namespaces and resource URIs match the classic
//! `winrs`/`Invoke-Command` cmd-shell transport (not PSRP).

use base64::Engine;
use quick_xml::Reader;
use quick_xml::events::Event;
use uuid::Uuid;

const RESOURCE_URI: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/cmd";
const ACTION_CREATE: &str = "http://schemas.xmlsoap.org/ws/2004/09/transfer/Create";
const ACTION_DELETE: &str = "http://schemas.xmlsoap.org/ws/2004/09/transfer/Delete";
const ACTION_COMMAND: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Command";
const ACTION_RECEIVE: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Receive";
const ACTION_SIGNAL: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Signal";
const SIGNAL_TERMINATE: &str =
    "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/signal/terminate";
const COMMAND_STATE_DONE: &str =
    "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/CommandState/Done";

fn header(to: &str, action: &str, shell_id: Option<&str>) -> String {
    let message_id = Uuid::new_v4();
    let selector_set = shell_id
        .map(|id| {
            format!(
                r#"<w:SelectorSet><w:Selector Name="ShellId">{id}</w:Selector></w:SelectorSet>"#
            )
        })
        .unwrap_or_default();
    format!(
        r#"<s:Header>
<a:To>{to}</a:To>
<a:ReplyTo><a:Address mustUnderstand="true">http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:Address></a:ReplyTo>
<w:MaxEnvelopeSize mustUnderstand="true">153600</w:MaxEnvelopeSize>
<a:MessageID>uuid:{message_id}</a:MessageID>
<w:Locale xml:lang="en-US" mustUnderstand="false"/>
<w:OperationTimeout>PT60S</w:OperationTimeout>
<a:Action mustUnderstand="true">{action}</a:Action>
<w:ResourceURI mustUnderstand="true">{RESOURCE_URI}</w:ResourceURI>
{selector_set}
</s:Header>"#
    )
}

fn envelope(to: &str, action: &str, shell_id: Option<&str>, body: &str) -> String {
    format!(
        r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing" xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd" xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
{header}
<s:Body>{body}</s:Body>
</s:Envelope>"#,
        header = header(to, action, shell_id)
    )
}

pub fn create_shell(to: &str) -> String {
    envelope(
        to,
        ACTION_CREATE,
        None,
        "<rsp:Shell><rsp:InputStreams>stdin</rsp:InputStreams><rsp:OutputStreams>stdout stderr</rsp:OutputStreams></rsp:Shell>",
    )
}

/// `command` and each entry of `args` are wrapped verbatim in their own
/// element; callers must not pass attacker-controlled text without XML
/// escaping (we only ever pass `powershell.exe` and a base64 blob).
pub fn run_command(to: &str, shell_id: &str, command: &str, args: &[&str]) -> String {
    let argument_elements = args
        .iter()
        .map(|arg| format!("<rsp:Arguments>{arg}</rsp:Arguments>"))
        .collect::<String>();
    let body = format!(
        "<rsp:CommandLine><rsp:Command>{command}</rsp:Command>{argument_elements}</rsp:CommandLine>"
    );
    envelope(to, ACTION_COMMAND, Some(shell_id), &body)
}

pub fn receive(to: &str, shell_id: &str, command_id: &str) -> String {
    let body = format!(
        r#"<rsp:Receive><rsp:DesiredStream CommandId="{command_id}">stdout stderr</rsp:DesiredStream></rsp:Receive>"#
    );
    envelope(to, ACTION_RECEIVE, Some(shell_id), &body)
}

pub fn signal_terminate(to: &str, shell_id: &str, command_id: &str) -> String {
    let body = format!(
        r#"<rsp:Signal CommandId="{command_id}"><rsp:Code>{SIGNAL_TERMINATE}</rsp:Code></rsp:Signal>"#
    );
    envelope(to, ACTION_SIGNAL, Some(shell_id), &body)
}

pub fn delete_shell(to: &str, shell_id: &str) -> String {
    envelope(to, ACTION_DELETE, Some(shell_id), "")
}

/// Local (namespace-stripped) tag name — Windows WS-Man responses can vary
/// which prefix they bind to which namespace, so every parser here matches
/// on local name rather than a fully-qualified name.
fn local_name(qname: &[u8]) -> &[u8] {
    match qname.iter().rposition(|&b| b == b':') {
        Some(index) => &qname[index + 1..],
        None => qname,
    }
}

pub fn parse_element_text(xml: &str, element_local_name: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut inside = false;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e))
                if local_name(e.name().as_ref()) == element_local_name.as_bytes() =>
            {
                inside = true;
            }
            Ok(Event::Text(t)) if inside => {
                return t.decode().ok().map(|s| s.into_owned());
            }
            Ok(Event::End(e)) if local_name(e.name().as_ref()) == element_local_name.as_bytes() => {
                inside = false;
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
}

pub struct ReceiveResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub done: bool,
    pub exit_code: Option<i32>,
}

/// Parse a `Receive` response: accumulate base64 `Stream` chunks by name and
/// detect the terminal `CommandState`/`ExitCode`. Malformed/undecodable
/// stream chunks are skipped rather than failing the whole response — a
/// single bad chunk shouldn't discard everything collected so far.
pub fn parse_receive_response(xml: &str) -> ReceiveResult {
    #[derive(Clone, Copy)]
    enum StreamName {
        Stdout,
        Stderr,
    }

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut result = ReceiveResult {
        stdout: Vec::new(),
        stderr: Vec::new(),
        done: false,
        exit_code: None,
    };
    // Only set on a real `Stream` *start* tag (an empty/self-closing `<Stream
    // .../>` end-of-stream marker carries no text, so it must not linger and
    // get mistaken for the next element's stream).
    let mut current_stream: Option<StreamName> = None;
    let mut pending_text: Option<String> = None;

    // `CommandState`'s `State` attribute lives on its opening tag, whether
    // that tag is self-closing (`Running`, no children) or a `Start` wrapping
    // an `ExitCode` child (`Done`) — check both event kinds identically.
    let mark_done_if_state_done = |e: &quick_xml::events::BytesStart<'_>,
                                   result: &mut ReceiveResult| {
        if local_name(e.name().as_ref()) == b"CommandState" {
            for attr in e.attributes().flatten() {
                if local_name(attr.key.as_ref()) == b"State"
                    && attr.value.as_ref() == COMMAND_STATE_DONE.as_bytes()
                {
                    result.done = true;
                }
            }
        }
    };

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = local_name(e.name().as_ref()).to_vec();
                if name == b"Stream" {
                    current_stream = e.attributes().flatten().find_map(|attr| {
                        (local_name(attr.key.as_ref()) == b"Name").then(|| {
                            if attr.value.as_ref() == b"stderr" {
                                StreamName::Stderr
                            } else {
                                StreamName::Stdout
                            }
                        })
                    });
                } else {
                    mark_done_if_state_done(&e, &mut result);
                }
            }
            Ok(Event::Empty(e)) => {
                mark_done_if_state_done(&e, &mut result);
            }
            Ok(Event::Text(t)) => {
                pending_text = t.decode().ok().map(|s| s.into_owned());
            }
            Ok(Event::End(e)) => {
                let name = local_name(e.name().as_ref()).to_vec();
                if name == b"Stream" {
                    if let (Some(stream), Some(text)) = (current_stream.take(), pending_text.take())
                        && let Ok(decoded) =
                            base64::engine::general_purpose::STANDARD.decode(text.trim())
                    {
                        match stream {
                            StreamName::Stdout => result.stdout.extend_from_slice(&decoded),
                            StreamName::Stderr => result.stderr.extend_from_slice(&decoded),
                        }
                    }
                } else if name == b"ExitCode"
                    && let Some(text) = pending_text.take()
                {
                    result.exit_code = text.trim().parse().ok();
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_builders_carry_the_right_action_and_selector() {
        let create = create_shell("http://10.0.0.5:5985/wsman");
        assert!(create.contains(ACTION_CREATE));
        assert!(!create.contains("SelectorSet")); // no ShellId yet

        let command = run_command(
            "http://10.0.0.5:5985/wsman",
            "shell-123",
            "powershell.exe",
            &["-NoProfile", "-EncodedCommand", "AAAA"],
        );
        assert!(command.contains(ACTION_COMMAND));
        assert!(command.contains(r#"Name="ShellId">shell-123<"#));
        assert!(command.contains("<rsp:Command>powershell.exe</rsp:Command>"));
        assert!(command.contains("<rsp:Arguments>-EncodedCommand</rsp:Arguments>"));

        let recv = receive("http://10.0.0.5:5985/wsman", "shell-123", "cmd-456");
        assert!(recv.contains(ACTION_RECEIVE));
        assert!(recv.contains(r#"CommandId="cmd-456""#));

        let signal = signal_terminate("http://10.0.0.5:5985/wsman", "shell-123", "cmd-456");
        assert!(signal.contains(ACTION_SIGNAL));
        assert!(signal.contains(SIGNAL_TERMINATE));

        let delete = delete_shell("http://10.0.0.5:5985/wsman", "shell-123");
        assert!(delete.contains(ACTION_DELETE));
    }

    #[test]
    fn parse_element_text_finds_shell_id_regardless_of_prefix() {
        let xml = r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:x="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
<s:Body><x:Shell><x:ShellId>ABCD-1234</x:ShellId></x:Shell></s:Body></s:Envelope>"#;
        assert_eq!(
            parse_element_text(xml, "ShellId"),
            Some("ABCD-1234".to_string())
        );
        assert_eq!(parse_element_text(xml, "CommandId"), None);
    }

    fn stream_chunk(name: &str, command_id: &str, text: &str) -> String {
        let encoded = base64::engine::general_purpose::STANDARD.encode(text);
        format!(r#"<rsp:Stream Name="{name}" CommandId="{command_id}">{encoded}</rsp:Stream>"#)
    }

    #[test]
    fn parse_receive_response_accumulates_stdout_and_stderr_separately() {
        let xml = format!(
            r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
<s:Body><rsp:ReceiveResponse>
{}{}{}
<rsp:CommandState State="{COMMAND_STATE_DONE}"><rsp:ExitCode>0</rsp:ExitCode></rsp:CommandState>
</rsp:ReceiveResponse></s:Body></s:Envelope>"#,
            stream_chunk("stdout", "cmd-1", "hello "),
            stream_chunk("stdout", "cmd-1", "world"),
            stream_chunk("stderr", "cmd-1", "oops"),
        );
        let result = parse_receive_response(&xml);
        assert_eq!(result.stdout, b"hello world");
        assert_eq!(result.stderr, b"oops");
        assert!(result.done);
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn parse_receive_response_not_done_while_running() {
        let xml = r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
<s:Body><rsp:ReceiveResponse>
<rsp:Stream Name="stdout" CommandId="cmd-1">aGVsbG8=</rsp:Stream>
<rsp:CommandState State="http://schemas.microsoft.com/wbem/wsman/1/windows/shell/CommandState/Running"/>
</rsp:ReceiveResponse></s:Body></s:Envelope>"#;
        let result = parse_receive_response(xml);
        assert_eq!(result.stdout, b"hello");
        assert!(!result.done);
        assert_eq!(result.exit_code, None);
    }

    #[test]
    fn parse_receive_response_ignores_empty_end_of_stream_marker() {
        // A self-closing Stream tag (end-of-stream marker) carries no text and
        // must not be mistaken for a data chunk on the next real element.
        let xml = format!(
            r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:rsp="http://schemas.microsoft.com/wbem/wsman/1/windows/shell">
<s:Body><rsp:ReceiveResponse>
{}
<rsp:Stream Name="stdout" CommandId="cmd-1" End="true"/>
<rsp:CommandState State="{COMMAND_STATE_DONE}"><rsp:ExitCode>1</rsp:ExitCode></rsp:CommandState>
</rsp:ReceiveResponse></s:Body></s:Envelope>"#,
            stream_chunk("stdout", "cmd-1", "only chunk"),
        );
        let result = parse_receive_response(&xml);
        assert_eq!(result.stdout, b"only chunk");
        assert!(result.done);
        assert_eq!(result.exit_code, Some(1));
    }
}
