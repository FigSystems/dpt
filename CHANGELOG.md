# Changelog

- Rename `fpkg` to `dpt`

- Make dpt be controlled by a central dpt configuration file

- Add `dev-env` command

- Fix issue with accessing base files.

- Hardlink ordinary files.

- Support multiple packages per DPTBUILD file

- Softer termination of programs when Ctrl-C is issued

- Proper dependency resolution

- Fix inaccurate exit codes for programs.

- Packages can be run by setting $0 of the dpt process to a different value.

- Add glue support.

- Switch all configuration files to RON instead of KDL.

- Allow versions with more then two decimal places

- Build packages with fakeroot

- Remove `gen-pkg` command as it doesn't work with fakeroot.

- Make `dev-env`s glues only have files from packages specified, not from the system.
