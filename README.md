# Unix V6++

Unix V6++ 是一个面向教学的操作系统内核。
项目最初基于 IA-32 架构用 C++ 实现，当前正在向 **RISC-V 64 位 + Rust** 迁移。

## 架构概览

| 目录 | 说明 |
|------|------|
| `src/` | Rust 内核（`no_std` + `staticlib`），入口：`_start` → `riscv64_rust_entry` |
| `programs/` | 用户态 C 程序（RISC-V 交叉编译） |
| `shell/` | Shell 程序 |
| `lib/` | 用户态 C 库（crt0、stdio、malloc、syscall 等） |
| `tools/` | 主机端 C++ 工具（filescanner + fs-editor），构建磁盘镜像 |
| `src/boot/` | 旧版 x86 引导扇区（RISC-V 目标不使用） |

RISC-V 64 位启动方式为 OpenSBI + qemu-system-riscv64。

## 环境要求

- Rust 工具链：**nightly-2026-04-07**
- RISC-V 交叉编译器：`riscv64-unknown-elf-gcc`
- QEMU：`qemu-system-riscv64`
- 文件系统编辑工具（`tools/filescanner`、`tools/fs-editor`）已包含在仓库中

## 构建与运行

```bash
make                        # 构建内核 + 用户程序 + 磁盘镜像
make qemu-riscv64           # 构建并启动 QEMU
make qemu-riscv64g          # 启动 QEMU（带 GDB 调试桩）
make check                  # cargo check（仅类型检查 Rust）
make clean                  # 清理所有构建产物

# 仅构建 Rust 内核（跳过磁盘镜像）
make build-rust

# 开启调试特性
make build-rust F=debug                     # 全部调试输出
make qemu-riscv64 F="debug_irq"  # 指定调试模块
```

### GDB 调试

```bash
# 终端 1：启动带调试桩的 QEMU
make qemu-riscv64g

# 终端 2：连接 GDB
make debug B=riscv64_rust_entry,bp2,bp3
```

## Rust 内核

- 目标三元组：`riscv64gc-unknown-none-elf`，使用 `-Z build-std` 编译 core/alloc
- 内核链接地址：`0x80200000`（链接脚本：`src/kernel-riscv64.ld`）
- panic 策略：abort；重定位：static；输出：`kernel.elf`
- 调试特性：`debug_irq`、`debug_timer`、`debug_scheduler`、`debug_proc`，`debug` 为全开

## 用户程序

- 交叉编译参数：`-march=rv64gc -mabi=lp64d -ffreestanding -nostdlib`
- 程序链接地址：`0x400000`
- `CROSS_COMPILE ?= riscv64-unknown-elf-`

## 磁盘镜像

`target/disk.img` 的构建流程：

1. 交叉编译 `programs/`、`shell/` 和 `lib/`
2. 将程序二进制复制到 `target/img-workspace/programs/`
3. 运行 `tools/filescanner` 扫描文件，管道输出到 `tools/fs-editor` 写入镜像
4. 同时将启动画面 BMP 文件复制到镜像中
