TARGET := riscv64gc-unknown-none-elf
MODE := release
APP_DIR := src/bin
TARGET_DIR := target/$(TARGET)/$(MODE)
# $(wildcard pattern) 是一个 Makefile 内置的函数，用于获取匹配指定模式的文件列表。
# pattern 是 $(APP_DIR)/*.rs，表示匹配目录 $(APP_DIR) 中所有扩展名为 .rs 的文件。
# 是一个以空格分隔的字符串，每个字符串代表一个文件的路径。
# src/bin/00write_a.rs src/bin/01write_b.rs src/bin/02write_c.rs
APPS := $(wildcard $(APP_DIR)/*.rs)
# $(patsubst pattern, replacement, text) 是一个 Makefile 内置的函数，用于将 text 中匹配到的 pattern 部分替换为 replacement。
# target/riscv64gc-unknown-none-elf/release/00write_a target/riscv64gc-unknown-none-elf/release/01write_b target/riscv64gc-unknown-none-elf/release/02write_c
ELFS := $(patsubst $(APP_DIR)/%.rs, $(TARGET_DIR)/%, $(APPS))
# target/riscv64gc-unknown-none-elf/release/00write_a.bin target/riscv64gc-unknown-none-elf/release/01write_b.bin target/riscv64gc-unknown-none-elf/release/02write_c.bin
BINS := $(patsubst $(APP_DIR)/%.rs, $(TARGET_DIR)/%.bin, $(APPS))
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# 依赖src/bin下的所有.rs文件
elf: $(APPS)
	@python3 build.py

binary: elf
	$(foreach elf, $(ELFS), $(OBJCOPY) $(elf) --strip-all -O binary $(patsubst $(TARGET_DIR)/%, $(TARGET_DIR)/%.bin, $(elf));)

build: binary

# @符号用于抑制命令的输出。当命令前面有@符号时，执行该命令时将不会在终端输出该命令的详细信息，只会执行命令本身
clean:
	@cargo clean

.PHONY: elf binary build clean