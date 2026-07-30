-- User-assignable (or collector-suggested) OS grouping for a host
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE hosts ADD COLUMN os_group TEXT;

COMMENT ON COLUMN hosts.os_group IS 'HostOsGroup variant name (Windows, Linux, LinuxDebian, Router, Switch, Unknown) - collector guidance, not a strict fingerprint';
