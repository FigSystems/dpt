# Changelog

- Make `/etc/fstab` be hardlinked inside the base environment.

- Add support for specifying enabled systemd services inside the dpt file.

- Make sure that the init process is PID 1 by `exec`ing dpt sub-processes.

- Add a `DPT Loaded` and `Starting init process!` banner upon bootup.

- Bind mount `/sys` inside containers.

- Fallback to `/dpt/dpt` if `/proc/self/exe` is not accesible.

- Link `/etc/mtab` to `/proc/self/mounts` for compatibility.

- Mount `/proc`, `/tmp`, `/sys`, `/sys/fs/cgroup`, and `/dev/pts` upon bootup. (Before init)

- Make unmounting significantly more robust.

- Use pivot_root instead of chroot for entering environments.

- Ignore the leading `-` on `argv[0]` if it is present, since it is added for login shells.
