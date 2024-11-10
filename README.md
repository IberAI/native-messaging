
# Native Messaging Helper for WebExtensions

This Rust crate provides a simple way to create, register, and manage native messaging host applications for WebExtensions. It includes cross-platform support for Chrome and Firefox, with functionalities to install, verify, and remove native messaging manifests, and enables asynchronous communication with WebExtensions.

## Features

- **Cross-platform Support:** Manage native messaging manifests for Chrome and Firefox on Linux and macOS.
- **Automatic Manifest Installation:** Easily create and install native messaging manifests for supported browsers.
- **Manifest Verification and Removal:** Check if a manifest is installed and remove it if necessary.
- **Asynchronous Message Handling:** Use event loops and asynchronous functions to encode, send, and receive messages.

## Table of Contents

- [Getting Started](#getting-started)
- [Installation](#installation)
- [Usage](#usage)
  - [Creating and Installing a Manifest](#creating-and-installing-a-manifest)
  - [Sending and Receiving Messages](#sending-and-receiving-messages)
  - [Verifying and Removing a Manifest](#verifying-and-removing-a-manifest)
- [Contributing](#contributing)
- [License](#license)

## Getting Started

### Prerequisites

- Rust
- Cargo package manager

### Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
native_messaging= "0.1.0"
```

## Usage

### Creating and Installing a Manifest

To create and install a native messaging manifest, use the `install::manifest::install` function from the `native_messaging` crate with the manifest's name, description, path to the native app, and the target browsers:

```rust
use native_messaging::install::manifest::install;

fn main() {
    install(
        "my_extension",
        "Description of my extension",
        "/path/to/extension/executable",
        &["chrome", "firefox"],
    ).expect("Failed to install the extension manifest");
}
```

This will create and install a manifest for the specified browsers if the path exists.

### Sending and Receiving Messages

To enable message communication with your WebExtension, use the `host` module functions to handle messaging operations such as `get_message` to read messages, `send_message` to send responses, and `event_loop` to manage asynchronous message handling.

#### Example: Reading a Message

```rust
use native_messaging::host::get_message;
use tokio;

#[tokio::main]
async fn main() {
    match get_message().await {
        Ok(message) => println!("Received: {}", message),
        Err(e) => eprintln!("Error receiving message: {}", e),
    }
}
```

#### Example: Sending a Message

```rust
use native_messaging::host::send_message;
use serde::Serialize;
use tokio;

#[derive(Serialize)]
struct MyMessage {
content: String}

#[tokio::main()]
async fn main() {
let message = MyMessage { content: "Hello, world!".to_string() };
if let Err(e) = send_message(&message).await {
eprintln!("Failed to send message: {}", e);
    }
}
```


#### Example: Running an Event Loop

To continuously receive messages and handle them, you can set up an `event_loop` using an async callback:

```rust
use native_messaging::host::{event_loop, send_message};
use tokio;

async fn handle_message(message: String) -> io::Result<()> {
    println!("Handling message: {}", message);
    Ok(())
}

#[tokio::main]
async fn main() {
    event_loop(handle_message).await;
}
```

### Verifying and Removing a Manifest

#### Verifying Manifest Installation

You can check if a manifest is installed using `verify` from the `install` module:

```rust
use native_messaging::install::manifest::verify;

fn main() {
    let installed = verify("my_extension").expect("Verification failed");
    if installed {
        println!("Manifest is installed.");
    } else {
        println!("Manifest is not installed.");
    }
}
```

#### Removing a Manifest

To remove a previously installed manifest, use `remove` from the `install` module:

```rust
use native_messaging::install::manifest::remove;

fn main() {
    remove("my_extension", &["chrome", "firefox"]).expect("Failed to remove extension");
}
```

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork this repository.
2. Create a new branch (`feature/my-feature`).
3. Commit your changes (`git commit -m 'Add feature'`).
4. Push to the branch (`git push origin feature/my-feature`).
5. Create a new Pull Request.

Feel free to report bugs or suggest features by opening an issue.

### Code of Conduct

Please follow the [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/version/2/0/code_of_conduct.html) in all your interactions with this project.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

This library makes managing native messaging easier, letting you focus on building your WebExtension instead of handling low-level manifest and messaging details. Happy coding!

