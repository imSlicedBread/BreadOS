#![no_main]
#![no_std]

use breados_boot::{
    BOOT_INFO_MAGIC, BOOT_INFO_VERSION, BootInfo, FramebufferInfo, MAX_MEMORY_REGIONS,
    MEMORY_AVAILABLE, MEMORY_BOOT_SERVICES, MEMORY_LOADER, MEMORY_MMIO, MEMORY_RESERVED,
    MEMORY_RUNTIME, MemoryMapInfo, MemoryRegion, PIXEL_FORMAT_BGRX8, PIXEL_FORMAT_RGBX8,
};
use core::{mem::size_of, ptr};
use uefi::{
    boot::{self, AllocateType, MemoryType},
    mem::memory_map::MemoryMap,
    prelude::*,
    proto::{
        console::gop::{GraphicsOutput, PixelFormat},
        media::file::{File, FileMode},
    },
};

const PAGE_SIZE: usize = 4096;
const KERNEL_STACK_PAGES: usize = 16;
const ELF_HEADER_SIZE: usize = 64;
const ELF_PROGRAM_HEADER_SIZE: usize = 56;

#[entry]
fn main() -> Status {
    if let Err(status) = unsafe { boot_os() } {
        serial("BreadOS loader: fatal error\r\n");
        return status;
    }
    Status::SUCCESS
}

unsafe fn boot_os() -> Result<(), Status> {
    serial_init();
    serial("BreadOS loader v001\r\n");
    let image = boot::image_handle();
    let mut fs = boot::get_image_file_system(image).map_err(|e| e.status())?;
    let mut root = fs.open_volume().map_err(|e| e.status())?;
    let kernel_name = uefi::cstr16!("\\KERNEL.ELF");
    let mut file = root
        .open(
            kernel_name,
            FileMode::Read,
            uefi::proto::media::file::FileAttribute::empty(),
        )
        .map_err(|e| e.status())?
        .into_regular_file()
        .ok_or(Status::LOAD_ERROR)?;
    let file_info = file
        .get_boxed_info::<uefi::proto::media::file::FileInfo>()
        .map_err(|e| e.status())?;
    let file_size = file_info.file_size() as usize;
    if file_size < ELF_HEADER_SIZE || file_size > 16 * 1024 * 1024 {
        return Err(Status::LOAD_ERROR);
    }
    let kernel_buffer =
        boot::allocate_pool(MemoryType::LOADER_DATA, file_size).map_err(|e| e.status())?;
    let kernel_bytes =
        unsafe { core::slice::from_raw_parts_mut(kernel_buffer.as_ptr(), file_size) };
    let read = file.read(kernel_bytes).map_err(|e| e.status())?;
    if read != file_size {
        return Err(Status::LOAD_ERROR);
    }
    drop(file);
    drop(root);
    drop(fs);

    let (entry, loaded_ranges) = unsafe { load_elf(kernel_bytes)? };
    serial("BreadOS loader: kernel segments validated and loaded\r\n");

    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>().map_err(|e| e.status())?;
    let mut gop =
        boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle).map_err(|e| e.status())?;
    let mode = gop.current_mode_info();
    let (width, height) = mode.resolution();
    let stride = mode.stride();
    let pixel_format = match mode.pixel_format() {
        PixelFormat::Rgb => PIXEL_FORMAT_RGBX8,
        PixelFormat::Bgr => PIXEL_FORMAT_BGRX8,
        _ => return Err(Status::UNSUPPORTED),
    };
    let fb = gop.frame_buffer().as_mut_ptr() as u64;
    let fb_len = gop.frame_buffer().size() as u64;
    let _ = loaded_ranges;
    drop(gop);

    let boot_info_ptr = boot::allocate_pool(MemoryType::LOADER_DATA, size_of::<BootInfo>())
        .map_err(|e| e.status())?
        .as_ptr()
        .cast::<BootInfo>();
    let region_buffer = boot::allocate_pool(
        MemoryType::LOADER_DATA,
        size_of::<MemoryRegion>() * MAX_MEMORY_REGIONS,
    )
    .map_err(|e| e.status())?
    .as_ptr()
    .cast::<MemoryRegion>();
    let stack = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        KERNEL_STACK_PAGES,
    )
    .map_err(|e| e.status())?;
    unsafe {
        ptr::write(
            boot_info_ptr,
            BootInfo {
                magic: BOOT_INFO_MAGIC,
                version: BOOT_INFO_VERSION,
                size: size_of::<BootInfo>() as u32,
                framebuffer: FramebufferInfo {
                    base: fb,
                    byte_len: fb_len,
                    width: width as u32,
                    height: height as u32,
                    stride_pixels: stride as u32,
                    pixel_format,
                },
                memory_map: MemoryMapInfo {
                    regions: region_buffer as u64,
                    count: 0,
                    descriptor_size: 0,
                    available_pages: 0,
                },
            },
        );
    }

    serial("BreadOS loader: leaving UEFI boot services\r\n");
    let memory_map = unsafe { boot::exit_boot_services(None) };
    let mut count = 0usize;
    let mut available = 0u64;
    for desc in memory_map.entries() {
        if count == MAX_MEMORY_REGIONS {
            break;
        }
        let kind = match desc.ty {
            MemoryType::CONVENTIONAL => {
                available += desc.page_count;
                MEMORY_AVAILABLE
            }
            MemoryType::LOADER_CODE | MemoryType::LOADER_DATA => MEMORY_LOADER,
            MemoryType::BOOT_SERVICES_CODE | MemoryType::BOOT_SERVICES_DATA => MEMORY_BOOT_SERVICES,
            MemoryType::RUNTIME_SERVICES_CODE | MemoryType::RUNTIME_SERVICES_DATA => MEMORY_RUNTIME,
            MemoryType::MMIO | MemoryType::MMIO_PORT_SPACE => MEMORY_MMIO,
            _ => MEMORY_RESERVED,
        };
        unsafe {
            ptr::write(
                region_buffer.add(count),
                MemoryRegion {
                    base: desc.phys_start,
                    pages: desc.page_count,
                    kind,
                    _reserved: 0,
                    attributes: desc.att.bits(),
                },
            )
        };
        count += 1;
    }
    unsafe {
        (*boot_info_ptr).memory_map.count = count as u32;
        (*boot_info_ptr).memory_map.available_pages = available;
        (*boot_info_ptr).memory_map.descriptor_size = size_of::<MemoryRegion>() as u32;
        let stack_top = stack.as_ptr() as u64 + (KERNEL_STACK_PAGES * PAGE_SIZE) as u64;
        core::arch::asm!(
            "mov rsp, rcx",
            "and rsp, -16",
            "push 0",
            "jmp rax",
            in("rax") entry,
            in("rdi") boot_info_ptr,
            in("rcx") stack_top,
            options(noreturn),
        );
    }
}

unsafe fn load_elf(bytes: &[u8]) -> Result<(u64, [(u64, u64); 16]), Status> {
    if &bytes[0..4] != b"\x7fELF"
        || bytes[4] != 2
        || bytes[5] != 1
        || bytes[6] != 1
        || u16::from_le_bytes([bytes[18], bytes[19]]) != 62
    {
        return Err(Status::LOAD_ERROR);
    }
    let entry = u64_at(bytes, 24)?;
    let phoff = u64_at(bytes, 32)? as usize;
    let phentsize = u16_at(bytes, 54)? as usize;
    let phnum = u16_at(bytes, 56)? as usize;
    if phentsize < ELF_PROGRAM_HEADER_SIZE
        || phnum > 16
        || phoff
            .checked_add(phentsize.checked_mul(phnum).ok_or(Status::LOAD_ERROR)?)
            .filter(|end| *end <= bytes.len())
            .is_none()
    {
        return Err(Status::LOAD_ERROR);
    }
    let mut ranges = [(0u64, 0u64); 16];
    let mut entry_valid = false;
    let mut loaded_count = 0usize;
    let mut total_bytes = 0u64;
    for i in 0..phnum {
        let p = phoff + i * phentsize;
        if u32_at(bytes, p)? != 1 {
            continue;
        }
        let flags = u32_at(bytes, p + 4)?;
        let offset = u64_at(bytes, p + 8)? as usize;
        let vaddr = u64_at(bytes, p + 16)?;
        let paddr = u64_at(bytes, p + 24)?;
        let filesz = u64_at(bytes, p + 32)? as usize;
        let memsz = u64_at(bytes, p + 40)? as usize;
        let p_align = u64_at(bytes, p + 48)?;
        let segment_end = paddr.checked_add(memsz as u64).ok_or(Status::LOAD_ERROR)?;
        if memsz == 0
            || memsz > 32 * 1024 * 1024
            || filesz > memsz
            || offset
                .checked_add(filesz)
                .filter(|end| *end <= bytes.len())
                .is_none()
            || paddr != vaddr
            || paddr < 0x100000
            || segment_end > 0x4000_0000
            || p_align > 1
                && (p_align & (p_align - 1) != 0
                    || (paddr as usize) % (p_align as usize) != offset % (p_align as usize))
            || flags & 3 == 3
        {
            return Err(Status::LOAD_ERROR);
        }
        let start = paddr & !((PAGE_SIZE as u64) - 1);
        let end = (paddr + memsz as u64)
            .checked_add(PAGE_SIZE as u64 - 1)
            .ok_or(Status::LOAD_ERROR)?
            & !((PAGE_SIZE as u64) - 1);
        for (prior_base, prior_pages) in ranges.iter().take(loaded_count) {
            let prior_end = prior_base + prior_pages * PAGE_SIZE as u64;
            if start < prior_end && *prior_base < end {
                return Err(Status::LOAD_ERROR);
            }
        }
        let pages = ((end - start) / PAGE_SIZE as u64) as usize;
        let dest =
            boot::allocate_pages(AllocateType::Address(start), MemoryType::LOADER_DATA, pages)
                .map_err(|e| e.status())?;
        unsafe {
            ptr::write_bytes(dest.as_ptr(), 0, pages * PAGE_SIZE);
            ptr::copy_nonoverlapping(bytes.as_ptr().add(offset), paddr as *mut u8, filesz);
        }
        ranges[loaded_count] = (start, pages as u64);
        loaded_count += 1;
        total_bytes = total_bytes
            .checked_add(memsz as u64)
            .ok_or(Status::LOAD_ERROR)?;
        if total_bytes > 64 * 1024 * 1024 {
            return Err(Status::LOAD_ERROR);
        }
        if flags & 1 != 0 && entry >= paddr && entry < segment_end {
            entry_valid = true;
        }
    }
    if loaded_count == 0 || !entry_valid {
        return Err(Status::LOAD_ERROR);
    }
    Ok((entry, ranges))
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16, Status> {
    let b = data.get(offset..offset + 2).ok_or(Status::LOAD_ERROR)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
fn u32_at(data: &[u8], offset: usize) -> Result<u32, Status> {
    let b = data.get(offset..offset + 4).ok_or(Status::LOAD_ERROR)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn u64_at(data: &[u8], offset: usize) -> Result<u64, Status> {
    let b = data.get(offset..offset + 8).ok_or(Status::LOAD_ERROR)?;
    Ok(u64::from_le_bytes([
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
    ]))
}

fn serial(message: &str) {
    for byte in message.bytes() {
        unsafe {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") byte, options(nomem, nostack, preserves_flags));
        }
    }
}

fn serial_init() {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") 0x3f9u16, in("al") 0u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3fbu16, in("al") 0x80u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") 1u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3f9u16, in("al") 0u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3fbu16, in("al") 3u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3fau16, in("al") 0xc7u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3fcu16, in("al") 0x0bu8, options(nomem, nostack, preserves_flags));
    }
}
