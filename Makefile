DESTDIR ?= /
prefix ?= /usr/local

none-specified: build-release

install: install-release

build-debug:
	cargo build

build-release:
	cargo build --release

install-makedpt:
	mkdir -p $(DESTDIR)/$(prefix)/bin
	cp tools/makedpt $(DESTDIR)/$(prefix)/bin/makedpt
	chmod +x $(DESTDIR)/$(prefix)/bin/makedpt

install-debug: install-makedpt
	mkdir -p $(DESTDIR)/$(prefix)/bin
	mkdir -p $(DESTDIR)/dpt
	cp -f target/x86_64-unknown-linux-musl/debug/dpt $(DESTDIR)/dpt/
	ln -srf $(DESTDIR)/dpt/dpt $(DESTDIR)/$(prefix)/bin/dpt 
	chown root:root $(DESTDIR)/dpt/dpt
	chmod u+s $(DESTDIR)/dpt/dpt

install-release: install-makedpt
	mkdir -p $(DESTDIR)/$(prefix)/bin
	mkdir -p $(DESTDIR)/dpt
	cp -f target/x86_64-unknown-linux-musl/release/dpt $(DESTDIR)/dpt/
	ln -srf $(DESTDIR)/dpt/dpt $(DESTDIR)/$(prefix)/bin/dpt 
	chown root:root $(DESTDIR)/dpt/dpt
	chmod u+s $(DESTDIR)/dpt/dpt


uninstall:
	rm -rf $(DESTDIR)/$(prefix)/bin/dpt
	rm -rf $(DESTDIR)/dpt/dpt
