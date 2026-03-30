use core::alloc::Layout;
use core::ptr::NonNull;
use crate::sysinfo::{MemoryRegions, MemoryType};

const ALLOCATOR_MIN_SIZE: usize = 8192;

struct Node {
    next: Option<NonNull<Node>>,
    prev: Option<NonNull<Node>>,
    size: usize,
    free: bool,
}

pub struct KernelMemoryAllocator {
    first: NonNull<Node>,
}

impl KernelMemoryAllocator {
    pub fn new(regions: &MemoryRegions) -> Result<Self, &'static str> {
        let largest_region = regions
            .iter()
            .filter(|region| region.region_type == MemoryType::Usable)
            .max_by_key(|region| region.size)
            .ok_or("no suitable memory regions found")?;

        let base_addr = largest_region.base_addr as usize;
        let aligned_base_addr = Self::align_up(base_addr, align_of::<Node>());
        let size = largest_region.size as usize - (aligned_base_addr - base_addr);

        if size < ALLOCATOR_MIN_SIZE {
            return Err("not enough memory");
        }

        let base_ptr = NonNull::new(aligned_base_addr as *mut Node)
            .ok_or("Largest region base address is null")?;

        let first_node = Node {
            next: None,
            prev: None,
            size,
            free: true,
        };

        unsafe { *base_ptr.as_ptr() = first_node };

        Ok(Self {
            first: base_ptr,
        })
    }

    fn align_up(addr: usize, alignment: usize) -> usize {
        (addr + alignment - 1) & !(alignment - 1)
    }

    fn value_addr(node_addr: usize) -> usize {
        node_addr + size_of::<Node>()
    }
}

impl KernelMemoryAllocator {
    pub fn alloc(&self, layout: Layout) -> Option<NonNull<u8>> {
        Some(self.find_suitable_node(layout.size(), layout.align())?.1)
    }

    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        if let Some(node) = self.search_for_node_containing_ptr(ptr) {
            self.free_node_or_coalesce(node);
        }
    }

    pub fn alloc_zeroed(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let (node, addr) = self.find_suitable_node(layout.size(), layout.align())?;
        unsafe { core::slice::from_raw_parts_mut(addr.as_ptr(), node.size) }
            .fill(0);
        Some(addr)
    }

    pub fn realloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize
    ) -> Option<NonNull<u8>> {
        if let Some(node) = self.search_for_node_containing_ptr(ptr) && new_size <= node.size {
            Some(ptr)
        } else {
            self.dealloc(ptr, layout);
            self.alloc(Layout::from_size_align(new_size, layout.align()).unwrap())
        }
    }
}

impl KernelMemoryAllocator {
    fn find_suitable_node<'a>(&self, size: usize, align: usize) -> Option<(&'a mut Node, NonNull<u8>)> {
        let mut current = Some(self.first);
        let mut found = false;
        while !found {
            if let Some(node_ptr) = current {
                let node = unsafe { &*node_ptr.as_ptr() };
                let node_addr = node_ptr.addr().get();
                let value_addr = Self::value_addr(node_addr);
                let aligned_value_addr = Self::align_up(value_addr, align);
                let leftover_size = node.size - (aligned_value_addr - value_addr);
                found = node.free && leftover_size >= size;
                if !found {
                    current = node.next;
                }
            } else {
                found = true;
            }
        }

        if let Some(node_ptr) = current {
            let node = unsafe { &mut *node_ptr.as_ptr() };
            node.free = false;

            let node_addr = node_ptr.addr().get();
            let aligned_value_addr = Self::align_up(Self::value_addr(node_addr), align);
            let aligned_value_ptr = NonNull::new(aligned_value_addr as *mut u8)?;

            self.reclaim_space_right(node, node_addr, aligned_value_addr + size);

            Some((node, aligned_value_ptr))
        } else {
            None
        }
    }

    fn reclaim_space_right(&self, node: &mut Node, node_addr: usize, mut start_from: usize) {
        start_from = Self::align_up(start_from, align_of::<Node>());
        if start_from < Self::value_addr(node_addr) ||
            node.size < start_from - Self::value_addr(node_addr) {
            return;
        }
        let remaining_space = node.size - (start_from - Self::value_addr(node_addr));
        if remaining_space <= size_of::<Node>() {
            return;
        }
        self.insert_node_after(node, start_from, remaining_space - size_of::<Node>(), true);
        node.size = start_from - Self::value_addr(node_addr);
    }

    fn insert_node_after(
        &self,
        node: &mut Node,
        addr: usize,
        size: usize,
        free: bool,
    ) -> Option<NonNull<Node>> {
        if size == 0 || !addr.is_multiple_of(align_of::<Node>()) { return None; }

        let new_node_ptr = NonNull::new(addr as *mut Node)?;
        let new_node = Node {
            next: node.next,
            prev: Some(NonNull::from_ref(node)),
            size,
            free,
        };
        unsafe { *new_node_ptr.as_ptr() = new_node };
        if let Some(next_ptr) = node.next {
            let next = unsafe { &mut *next_ptr.as_ptr() };
            next.prev = Some(new_node_ptr);
        }
        node.next = Some(new_node_ptr);

        Some(new_node_ptr)
    }

    fn search_for_node_containing_ptr<'a>(&self, ptr: NonNull<u8>) -> Option<&'a mut Node> {
        let addr = ptr.addr().get();
        let mut current = Some(self.first);
        let mut found = false;
        while !found {
            if let Some(node_ptr) = current {
                let value_addr = Self::value_addr(node_ptr.addr().get());
                let node = unsafe { &*node_ptr.as_ptr() };
                found = !node.free && value_addr <= addr && addr < value_addr + node.size;
                if !found {
                    current = node.next;
                }
            } else {
                found = true;
            }
        }
        current.map(|node_ptr| unsafe { &mut *node_ptr.as_ptr() })
    }

    fn free_node_or_coalesce(&self, node: &mut Node) {
        if let Some(prev_ptr) = node.prev {
            let prev = unsafe { &mut *prev_ptr.as_ptr() };
            if prev.free {
                self.merge_right(prev, node);
            }
        }
        if let Some(next_ptr) = node.next {
            let next = unsafe { &mut *next_ptr.as_ptr() };
            if next.free {
                self.merge_right(node, next);
            }
        }
        node.free = true;
    }

    fn merge_right(&self, left: &mut Node, right: &mut Node) {
        let left_addr = left as *mut Node as usize;
        let right_addr = right as *mut Node as usize;
        left.size = Self::value_addr(right_addr) + right.size - Self::value_addr(left_addr);
        left.next = right.next;
        if let Some(right_next_ptr) = right.next {
            let right_next = unsafe { &mut *right_next_ptr.as_ptr() };
            right_next.prev = Some(NonNull::from_ref(left));
        }
    }
}
