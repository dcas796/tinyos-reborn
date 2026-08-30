mod ahci;

use alloc::boxed::Box;
use crate::io::{hal, pci};
use crate::io::disk::ahci::AhciController;

pub fn find_disk_controllers(
    endpoints: &[pci::Endpoint]
) -> impl Iterator<Item = Box<dyn hal::disk::DiskController<Disk=impl hal::disk::Disk>>> {
    endpoints
        .iter()
        .filter_map(controller_from_endpoint)
}

fn controller_from_endpoint(
    endpoint: &pci::Endpoint
) -> Option<Box<dyn hal::disk::DiskController<Disk=impl hal::disk::Disk>>> {
    let pci::FunctionIdentifier { class, subclass, prog_if } = endpoint.func_identifier;

    match (class, subclass, prog_if) {
        (0x01, 0x06, 0x01) => Some(Box::new(AhciController::new(*endpoint))),
        _ => None,
    }
}
