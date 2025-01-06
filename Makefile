build:
	cargo build -r --target x86_64-unknown-linux-gnu	

	cargo build -r --target aarch64-apple-darwin
	cargo build -r --target x86_64-apple-darwin

	cargo build -r --target x86_64-pc-windows-gnu
	cargo build -r --target x86_64-pc-windows-msvc
