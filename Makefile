DESTDIR ?= /usr/local

none-specified: install-release

install-common:
	sudo cp tools/makefpkg $(DESTDIR)/bin/makefpkg
	sudo chmod +x $(DESTDIR)/bin/makefpkg

install-debug: install-common
	cargo build
	sudo mkdir -p $(DESTDIR)/bin
	sudo cp -f target/x86_64-unknown-linux-musl/debug/fpkg $(DESTDIR)/bin/
	sudo chown root:root $(DESTDIR)/bin/fpkg
	sudo chmod u+s $(DESTDIR)/bin/fpkg

install-release: install-common
	cargo build --release
	sudo mkdir -p $(DESTDIR)/bin
	sudo cp -f target/x86_64-unknown-linux-musl/release/fpkg $(DESTDIR)/bin/
	sudo chown root:root $(DESTDIR)/bin/fpkg
	sudo chmod u+s $(DESTDIR)/bin/fpkg
