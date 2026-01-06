# Pervie

Pervie is a B L A Z I N G L Y F A S T TUI in rust for reformatting storage drives and flashing ISOs from remote servers to your local drive.

Why is it called Pervie? Because it's good at flashing ( ͡° ͜ʖ ͡°)

## Features

- Reformat storage drives into exFAT, FAT32, or NTFS.
- Safely unmount and eject storage drives.
- Flash ISOs from remote servers to your usb drive. No need to download the ISO to your computer first.
- Root drive is protected from changes.
- Mac and Linux support.

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/logscore/pervie/main/install.sh | bash
```

## Usage

```bash
pervie
```

Pervie needs root permissions for some operations. We handle this automatically. If you get an error, try running the command again with `sudo`.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

This project is licensed under the WTFPL - see the [LICENSE](./LICENSE) file for details.
