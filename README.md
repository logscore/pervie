# Pervie

Pervie is a B L A Z I N G L Y F A S T TUI in rust for reformatting storage drives and flashing ISOs from remote servers to your local drive.

Why is it called Pervie? Because it's good at flashing ( ͡° ͜ʖ ͡°)

## Features

- Reformat storage drives into exFAT, FAT32, or NTFS.
- Safely unmount and eject storage drives.
- Flash ISOs from remote servers to your usb drive. No need to download the ISO to your computer first.
- Mac and Linux (sort of) support.

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/logscore/pervie/main/install.sh | bash
```

## Usage

```bash
pervie
```

Some operations may fail if you don't have administrator privileges. If you get an error, try running the command again with `sudo`.

> Note that the Linux version isn't in a stable condition and might not work. Running the command again usually fixes it.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

This project is licensed under the WTFPL - see the [LICENSE](./LICENSE) file for details.
