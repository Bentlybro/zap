# ⚡ Zap

> Dead simple end-to-end encrypted file transfers from your terminal

Zap is a blazingly fast CLI/TUI tool for sending files between devices with end-to-end encryption. No setup, no accounts, no cloud—just simple word codes and direct transfers.

```bash
# On sender:
zap send myfile.zip
# => Transfer code: alpha-bravo-charlie

# On receiver:
zap receive alpha-bravo-charlie
```

## ✨ Features

- 🔒 **End-to-end encryption** using SPAKE2 key exchange + ChaCha20-Poly1305
- 🎯 **Simple word codes** - no need to remember IPs or ports
- 🚀 **LAN-first discovery** via mDNS (zero config on same network)
- 🌐 **Remote transfers** with direct TCP fallback
- 📊 **Beautiful TUI** with progress bars and speed indicators
- 🔧 **Pipe support** for streaming data
- 📁 **Directory transfers** (automatically tar'd and streamed)
- ⏸️ **Resumable transfers** (coming soon)

## 🚀 Installation

### From source (Rust required)

```bash
cargo install --git https://github.com/Bentlybro/zap
```

### From binaries

Download the latest release from [GitHub Releases](https://github.com/Bentlybro/zap/releases)

## 📖 Usage

### Send a file

```bash
# Send a file (generates a random code)
zap send myfile.zip

# Send with custom code
zap send myfile.zip --code my-secret-code

# Send from stdin
cat data.txt | zap send
```

### Receive a file

```bash
# Receive a file
zap receive alpha-bravo-charlie

# Receive to specific path
zap receive alpha-bravo-charlie --output downloads/myfile.zip

# Receive to stdout
zap receive alpha-bravo-charlie > myfile.zip
```

### Options

```bash
# Use simple progress bars instead of TUI
zap send myfile.zip --no-tui

# Use custom port
zap send myfile.zip --port 8080

# Verbose output
zap send myfile.zip --verbose
```

## 🔐 Security

Zap uses industry-standard cryptography:

- **Key exchange**: SPAKE2 (Password-Authenticated Key Exchange)
- **Encryption**: ChaCha20-Poly1305 (AEAD cipher)
- **Transfer codes**: Random words from a curated wordlist

Your files are encrypted **before** they leave your device and decrypted **only** on the receiver's device. The transfer code is never sent over the network—it's only used to derive the encryption keys.

## 🎯 Comparison

| Feature | Zap | Magic Wormhole | croc | wetransfer |
|---------|-----|----------------|------|------------|
| E2E Encryption | ✅ | ✅ | ✅ | ❌ |
| No relay needed (LAN) | ✅ | ❌ | ❌ | ❌ |
| Word codes | ✅ | ✅ | ✅ | ❌ |
| TUI | ✅ | ❌ | ❌ | ❌ |
| Resumable | 🚧 | ✅ | ✅ | ❌ |
| Pipe support | ✅ | ✅ | ❌ | ❌ |

## 🛠️ How It Works

1. **Sender** starts `zap send file.zip` and gets a transfer code
2. **Receiver** runs `zap receive alpha-bravo-charlie`
3. **Discovery**: 
   - On LAN: mDNS automatically discovers the sender
   - Remote: Manual IP entry or relay server (coming soon)
4. **Handshake**: SPAKE2 key exchange using the transfer code
5. **Transfer**: File is encrypted, chunked, and streamed to receiver
6. **Verification**: Checksum validates file integrity

## 🚧 Roadmap

- [x] Basic send/receive over TCP
- [x] E2E encryption
- [x] Word code generation
- [x] TUI with progress bars
- [ ] mDNS LAN discovery
- [ ] NAT traversal / hole punching
- [ ] Resumable transfers
- [ ] Relay server for NAT-to-NAT transfers
- [ ] Multiple file transfers
- [ ] QR code generation for mobile
- [ ] Web UI for easier sharing

## 🤝 Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## 📝 License

MIT License - see [LICENSE](LICENSE) for details

## 🙏 Credits

Inspired by:
- [Magic Wormhole](https://github.com/magic-wormhole/magic-wormhole)
- [croc](https://github.com/schollz/croc)
- [ffsend](https://github.com/timvisee/ffsend)

Built with ❤️ using Rust 🦀
