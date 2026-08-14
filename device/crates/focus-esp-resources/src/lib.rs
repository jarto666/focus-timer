#![no_std]

/// One instantaneous view of ESP-IDF's allocator and the calling FreeRTOS
/// task. ESP-IDF reports all values in bytes for this target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceSnapshot {
    pub heap_free_8bit: usize,
    pub heap_minimum_8bit: usize,
    pub current_stack_minimum_free: u32,
}

/// Reads ESP-IDF's process-wide 8-bit heap counters and the calling task's
/// stack high-water mark.
#[must_use]
#[allow(unsafe_code)]
pub fn snapshot() -> ResourceSnapshot {
    // SAFETY: These ESP-IDF inspection APIs only read allocator/task metadata.
    // A null task handle is explicitly documented to mean the calling task;
    // no pointer is dereferenced by application code and no ownership crosses
    // the FFI boundary.
    unsafe {
        ResourceSnapshot {
            heap_free_8bit: esp_idf_svc::sys::heap_caps_get_free_size(
                esp_idf_svc::sys::MALLOC_CAP_8BIT,
            ),
            heap_minimum_8bit: esp_idf_svc::sys::heap_caps_get_minimum_free_size(
                esp_idf_svc::sys::MALLOC_CAP_8BIT,
            ),
            current_stack_minimum_free: esp_idf_svc::sys::uxTaskGetStackHighWaterMark(
                core::ptr::null_mut(),
            ),
        }
    }
}
