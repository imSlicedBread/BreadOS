#![no_std]

pub const BOOT_INFO_MAGIC: u64 = 0x4252_4541_444f_5301;
pub const BOOT_INFO_VERSION: u32 = 1;
pub const MAX_MEMORY_REGIONS: usize = 512;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct BootInfo {
    pub magic: u64,
    pub version: u32,
    pub size: u32,
    pub framebuffer: FramebufferInfo,
    pub memory_map: MemoryMapInfo,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FramebufferInfo {
    pub base: u64,
    pub byte_len: u64,
    pub width: u32,
    pub height: u32,
    pub stride_pixels: u32,
    pub pixel_format: u32,
}

pub const PIXEL_FORMAT_RGBX8: u32 = 1;
pub const PIXEL_FORMAT_BGRX8: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryMapInfo {
    pub regions: u64,
    pub count: u32,
    pub descriptor_size: u32,
    pub available_pages: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryRegion {
    pub base: u64,
    pub pages: u64,
    pub kind: u32,
    pub _reserved: u32,
    pub attributes: u64,
}

pub const MEMORY_AVAILABLE: u32 = 1;
pub const MEMORY_RESERVED: u32 = 2;
pub const MEMORY_LOADER: u32 = 3;
pub const MEMORY_BOOT_SERVICES: u32 = 4;
pub const MEMORY_RUNTIME: u32 = 5;
pub const MEMORY_MMIO: u32 = 6;

pub const fn valid_header(info: &BootInfo) -> bool {
    info.magic == BOOT_INFO_MAGIC
        && info.version == BOOT_INFO_VERSION
        && info.size as usize == core::mem::size_of::<BootInfo>()
}

pub fn valid_framebuffer(fb: &FramebufferInfo) -> bool {
    if fb.base == 0
        || fb.width == 0
        || fb.height == 0
        || fb.stride_pixels < fb.width
        || fb.pixel_format != PIXEL_FORMAT_RGBX8 && fb.pixel_format != PIXEL_FORMAT_BGRX8
    {
        return false;
    }
    match (fb.stride_pixels as u64)
        .checked_mul(fb.height as u64)
        .and_then(|n| n.checked_mul(4))
    {
        Some(bytes) => bytes <= fb.byte_len && (fb.base as u64).checked_add(bytes).is_some(),
        None => false,
    }
}

pub fn valid_memory_map(map: &MemoryMapInfo) -> bool {
    if map.count as usize > MAX_MEMORY_REGIONS
        || map.descriptor_size as usize != core::mem::size_of::<MemoryRegion>()
        || map.count > 0
            && (map.regions == 0
                || map.regions as usize % core::mem::align_of::<MemoryRegion>() != 0)
    {
        return false;
    }
    true
}

pub fn valid_memory_regions(map: &MemoryMapInfo, regions: &[MemoryRegion]) -> bool {
    if regions.len() != map.count as usize || !valid_memory_map(map) {
        return false;
    }
    let mut available = 0u64;
    for (index, region) in regions.iter().enumerate() {
        if region.pages == 0 || region.base & 0xfff != 0 {
            return false;
        }
        let Some(end) = region
            .pages
            .checked_mul(4096)
            .and_then(|bytes| region.base.checked_add(bytes))
        else {
            return false;
        };
        if region.kind == MEMORY_AVAILABLE {
            let Some(next) = available.checked_add(region.pages) else {
                return false;
            };
            available = next;
        }
        for prior in &regions[..index] {
            let Some(prior_end) = prior
                .pages
                .checked_mul(4096)
                .and_then(|bytes| prior.base.checked_add(bytes))
            else {
                return false;
            };
            if region.base < prior_end && prior.base < end {
                return false;
            }
        }
    }
    available == map.available_pages
}

pub fn clipped_rect(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    screen_w: usize,
    screen_h: usize,
) -> Option<(usize, usize, usize, usize)> {
    if x >= screen_w || y >= screen_h {
        return None;
    }
    let right = match x.checked_add(width) {
        Some(v) => v.min(screen_w),
        None => screen_w,
    };
    let bottom = match y.checked_add(height) {
        Some(v) => v.min(screen_h),
        None => screen_h,
    };
    Some((x, y, right - x, bottom - y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framebuffer_requires_supported_format_and_full_stride() {
        let good = FramebufferInfo {
            base: 0x1000,
            byte_len: 64 * 32 * 4,
            width: 64,
            height: 32,
            stride_pixels: 64,
            pixel_format: PIXEL_FORMAT_BGRX8,
        };
        assert!(valid_framebuffer(&good));
        assert!(!valid_framebuffer(&FramebufferInfo {
            stride_pixels: 63,
            ..good
        }));
        assert!(!valid_framebuffer(&FramebufferInfo {
            pixel_format: 99,
            ..good
        }));
        assert!(!valid_framebuffer(&FramebufferInfo {
            byte_len: 10,
            ..good
        }));
        assert!(!valid_framebuffer(&FramebufferInfo {
            base: u64::MAX - 4,
            ..good
        }));
    }

    #[test]
    fn header_and_memory_map_versions_are_checked() {
        let info = BootInfo {
            magic: BOOT_INFO_MAGIC,
            version: BOOT_INFO_VERSION,
            size: core::mem::size_of::<BootInfo>() as u32,
            ..BootInfo::default()
        };
        assert!(valid_header(&info));
        assert!(!valid_header(&BootInfo { version: 2, ..info }));
        let map = MemoryMapInfo {
            regions: 0x1000,
            count: 1,
            descriptor_size: size_of::<MemoryRegion>() as u32,
            available_pages: 10,
        };
        assert!(valid_memory_map(&map));
        assert!(!valid_memory_map(&MemoryMapInfo { regions: 0, ..map }));
        assert!(!valid_memory_map(&MemoryMapInfo {
            count: (MAX_MEMORY_REGIONS + 1) as u32,
            ..map
        }));
    }

    #[test]
    fn rectangles_clip_without_wrapping() {
        assert_eq!(
            clipped_rect(90, 90, 20, 20, 100, 100),
            Some((90, 90, 10, 10))
        );
        assert_eq!(clipped_rect(100, 0, 1, 1, 100, 100), None);
        assert_eq!(
            clipped_rect(99, 0, usize::MAX, 1, 100, 100),
            Some((99, 0, 1, 1))
        );
    }

    #[test]
    fn memory_regions_are_aligned_nonoverlapping_and_accounted() {
        let regions = [
            MemoryRegion {
                base: 0x1000,
                pages: 4,
                kind: MEMORY_AVAILABLE,
                _reserved: 0,
                attributes: 0,
            },
            MemoryRegion {
                base: 0x5000,
                pages: 2,
                kind: MEMORY_RESERVED,
                _reserved: 0,
                attributes: 0,
            },
        ];
        let map = MemoryMapInfo {
            regions: 0x2000,
            count: 2,
            descriptor_size: size_of::<MemoryRegion>() as u32,
            available_pages: 4,
        };
        assert!(valid_memory_regions(&map, &regions));
        let overlap = [
            regions[0],
            MemoryRegion {
                base: 0x4000,
                ..regions[1]
            },
        ];
        assert!(!valid_memory_regions(&map, &overlap));
        let wrong_total = MemoryMapInfo {
            available_pages: 5,
            ..map
        };
        assert!(!valid_memory_regions(&wrong_total, &regions));
    }
}
