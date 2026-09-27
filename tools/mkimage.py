#!/usr/bin/env python3
"""Create the project's fixed-size FAT12 UEFI boot image (no host tools required)."""
from __future__ import annotations

import argparse
import struct
from pathlib import Path

SECTOR = 512
TOTAL_SECTORS = 2880
FAT_SECTORS = 9
ROOT_ENTRIES = 224
ROOT_SECTORS = ROOT_ENTRIES * 32 // SECTOR
DATA_START = 1 + 2 * FAT_SECTORS + ROOT_SECTORS
IMAGE_SIZE = TOTAL_SECTORS * SECTOR


def short_name(name: str) -> bytes:
    if name == ".":
        return b".          "
    if name == "..":
        return b"..         "
    stem, dot, ext = name.partition(".")
    if len(stem) > 8 or len(ext) > 3 or not stem:
        raise ValueError(f"not an 8.3 FAT name: {name}")
    return stem.upper().ljust(8).encode("ascii") + ext.upper().ljust(3).encode("ascii")


def directory_entry(name: str, attr: int, cluster: int, size: int) -> bytes:
    entry = bytearray(32)
    entry[:11] = short_name(name)
    entry[11] = attr
    struct.pack_into("<H", entry, 26, cluster)
    struct.pack_into("<I", entry, 28, size)
    return bytes(entry)


def set_fat12(fat: bytearray, cluster: int, value: int) -> None:
    offset = cluster + cluster // 2
    value &= 0xFFF
    if cluster & 1:
        fat[offset] = (fat[offset] & 0x0F) | ((value << 4) & 0xF0)
        fat[offset + 1] = value >> 4
    else:
        fat[offset] = value & 0xFF
        fat[offset + 1] = (fat[offset + 1] & 0xF0) | (value >> 8)


def build(efi_path: Path, kernel_path: Path, output: Path) -> None:
    efi = efi_path.read_bytes()
    kernel = kernel_path.read_bytes()
    image = bytearray(IMAGE_SIZE)

    boot = bytearray(SECTOR)
    boot[:3] = b"\xEB\x3C\x90"
    boot[3:11] = b"BREADOS "
    struct.pack_into("<HBHBHHBHHHII", boot, 11, SECTOR, 1, 1, 2, ROOT_ENTRIES, TOTAL_SECTORS, 0xF0, FAT_SECTORS, 18, 2, 0, 0)
    boot[36:39] = bytes((0, 0, 0x29))
    struct.pack_into("<I", boot, 39, 0xB0EAD001)
    boot[43:54] = b"BREADOS    "
    boot[54:62] = b"FAT12   "
    boot[510:512] = b"\x55\xAA"
    image[:SECTOR] = boot

    fat = bytearray(FAT_SECTORS * SECTOR)
    fat[:3] = b"\xF0\xFF\xFF"
    free_cluster = 2

    def write_file(data: bytes) -> int:
        nonlocal free_cluster
        needed = max(1, (len(data) + SECTOR - 1) // SECTOR)
        first = free_cluster
        clusters = list(range(first, first + needed))
        if clusters[-1] >= 0xFF0:
            raise ValueError("boot files exceed FAT12 image capacity")
        for index, cluster in enumerate(clusters):
            set_fat12(fat, cluster, clusters[index + 1] if index + 1 < len(clusters) else 0xFFF)
            sector = DATA_START + cluster - 2
            chunk = data[index * SECTOR:(index + 1) * SECTOR]
            start = sector * SECTOR
            image[start:start + len(chunk)] = chunk
        free_cluster += needed
        return first

    efi_cluster = write_file(efi)
    kernel_cluster = write_file(kernel)
    dir_cluster = free_cluster
    free_cluster += 1
    set_fat12(fat, dir_cluster, 0xFFF)
    for fat_index in range(2):
        start = (1 + fat_index * FAT_SECTORS) * SECTOR
        image[start:start + len(fat)] = fat

    root_start = (1 + 2 * FAT_SECTORS) * SECTOR
    image[root_start:root_start + 32] = directory_entry("EFI", 0x10, dir_cluster, 0)
    image[root_start + 32:root_start + 64] = directory_entry("KERNEL.ELF", 0x20, kernel_cluster, len(kernel))
    dir_start = (DATA_START + dir_cluster - 2) * SECTOR
    image[dir_start:dir_start + 32] = directory_entry(".", 0x10, dir_cluster, 0)
    image[dir_start + 32:dir_start + 64] = directory_entry("..", 0x10, 0, 0)
    image[dir_start + 64:dir_start + 96] = directory_entry("BOOT", 0x10, dir_cluster + 1, 0)
    boot_dir_cluster = dir_cluster + 1
    set_fat12(fat, boot_dir_cluster, 0xFFF)
    for fat_index in range(2):
        start = (1 + fat_index * FAT_SECTORS) * SECTOR
        image[start:start + len(fat)] = fat
    boot_dir_start = (DATA_START + boot_dir_cluster - 2) * SECTOR
    image[boot_dir_start:boot_dir_start + 32] = directory_entry(".", 0x10, boot_dir_cluster, 0)
    image[boot_dir_start + 32:boot_dir_start + 64] = directory_entry("..", 0x10, dir_cluster, 0)
    image[boot_dir_start + 64:boot_dir_start + 96] = directory_entry("BOOTX64.EFI", 0x20, efi_cluster, len(efi))
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(image)
    print(f"created {output} ({len(image)} bytes), EFI {len(efi)} bytes, kernel {len(kernel)} bytes")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--efi", type=Path, required=True)
    parser.add_argument("--kernel", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    build(args.efi, args.kernel, args.output)


if __name__ == "__main__":
    main()
