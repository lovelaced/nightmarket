// Shared panic handler for all contracts
// This should be included in each contract via include!("../../../shared/src/panic_handler.rs")

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::asm!("unimp");
        core::hint::unreachable_unchecked();
    }
}
