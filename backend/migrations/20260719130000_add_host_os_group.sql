-- User-assignable (or collector-suggested) OS grouping for a host

ALTER TABLE hosts ADD COLUMN os_group TEXT;

COMMENT ON COLUMN hosts.os_group IS 'HostOsGroup variant name (Windows, Linux, LinuxDebian, Router, Switch, Unknown) - collector guidance, not a strict fingerprint';
