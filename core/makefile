.DEFAULT_GOAL := help
PROJECT_NAME=$(shell basename "$(PWD)")
SOURCES=$(sort $(wildcard ./src/*.rs ./src/**/*.rs))

OS_NAME=$(shell uname | tr '[:upper:]' '[:lower:]')
PATH := $(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin:$(PATH)

ANDROID_AARCH64_LINKER=$(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin/aarch64-linux-android30-clang
ANDROID_ARMV7_LINKER=$(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin/armv7a-linux-androideabi30-clang
ANDROID_I686_LINKER=$(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin/i686-linux-android30-clang
ANDROID_X86_64_LINKER=$(ANDROID_NDK_HOME)/toolchains/llvm/prebuilt/$(OS_NAME)-x86_64/bin/x86_64-linux-android30-clang

SHELL := /bin/bash

# ##############################################################################
# # GENERAL
# ##############################################################################

.PHONY: help
help: makefile
	@echo
	@echo " Available actions in "$(PROJECT_NAME)":"
	@echo
	@sed -n 's/^##//p' $< | column -t -s ':' |  sed -e 's/^/ /'
	@echo

## init: Install missing dependencies.
.PHONY: init
init:
	rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
	cargo install cbindgen

## :

# ##############################################################################
# # RECIPES
# ##############################################################################

## all: Compile iOS, Android and bindings targets
all: android bindings

## android: Compile the android targets (arm64, armv7 and i686)
android: target/aarch64-linux-android/release/libnekoton.so target/armv7-linux-androideabi/release/libnekoton.so target/i686-linux-android/release/libnekoton.so target/x86_64-linux-android/release/libnekoton.so

target/aarch64-linux-android/release/libnekoton.so: $(SOURCES) ndk-home
	CC_aarch64_linux_android=$(ANDROID_AARCH64_LINKER) \
	CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$(ANDROID_AARCH64_LINKER) \
		cargo build --target aarch64-linux-android --release
	@echo "[DONE] $@"

target/armv7-linux-androideabi/release/libnekoton.so: $(SOURCES) ndk-home
	CC_armv7_linux_androideabi=$(ANDROID_ARMV7_LINKER) \
	CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER=$(ANDROID_ARMV7_LINKER) \
		cargo build --target armv7-linux-androideabi --release
	@echo "[DONE] $@"

target/i686-linux-android/release/libnekoton.so: $(SOURCES) ndk-home
	CC_i686_linux_android=$(ANDROID_I686_LINKER) \
	CARGO_TARGET_I686_LINUX_ANDROID_LINKER=$(ANDROID_I686_LINKER) \
		cargo  build --target i686-linux-android --release
	@echo "[DONE] $@"

target/x86_64-linux-android/release/libnekoton.so: $(SOURCES) ndk-home
	CC_x86_64_linux_android=$(ANDROID_X86_64_LINKER) \
	CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=$(ANDROID_X86_64_LINKER) \
		cargo build --target x86_64-linux-android --release
	@echo "[DONE] $@"

.PHONY: ndk-home
ndk-home:
	@if [ ! -d "${ANDROID_NDK_HOME}" ] ; then \
		echo "Error: Please, set the ANDROID_NDK_HOME env variable to point to your NDK folder" ; \
		exit 1 ; \
	fi

## bindings: Generate the .h file
bindings: target/bindings.h

target/bindings.h: $(SOURCES)
	cbindgen ./ -c cbindgen.toml | uniq > $@
	@echo "[DONE] $@"

## :

# ##############################################################################
# # OTHER
# ##############################################################################

## clean:
.PHONY: clean
clean:
	cargo clean
	rm -f target/bindings.h target/bindings.src.h

## test:
.PHONY: test
test:
	cargo test
