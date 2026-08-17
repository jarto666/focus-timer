#![cfg(feature = "tft-renderer")]

use focus_core::{AppSnapshot, ViewState, default_catalog};
use focus_firmware::{
    presentation::{TFT_PROGRESS_MAX, TftBackgroundMode, tft_view},
    tft_render::{TFT_BYTES_PER_ROW, TFT_ROWS_PER_TRANSFER, TFT_TRANSFER_BYTES, TftRenderer},
};

fn view(state: ViewState, remaining_ms: u64) -> focus_firmware::presentation::TftView {
    tft_view(
        &AppSnapshot {
            state,
            preset: default_catalog().preset(2),
            remaining_ms,
        },
        1,
    )
}

#[test]
fn palette_is_bounded_rgb565_and_non_decreasing() {
    let renderer = TftRenderer::new();
    let mut previous = renderer.palette_color(0);
    for index in 1..=u8::MAX {
        let color = renderer.palette_color(index);
        assert!(color >= previous);
        previous = color;
    }
}

#[test]
fn noise_sampling_wraps_without_a_texture_seam() {
    let renderer = TftRenderer::new();
    for phase in [0, 17, 999] {
        assert_eq!(
            renderer.wrapped_sample(0, 11, phase),
            renderer.wrapped_sample(128, 11, phase)
        );
        assert_eq!(
            renderer.wrapped_sample(23, 0, phase),
            renderer.wrapped_sample(23, 128, phase)
        );
    }
}

#[test]
fn foreground_progress_wins_over_background() {
    let renderer = TftRenderer::new();
    let mut output = [0_u8; TFT_TRANSFER_BYTES];
    let running = view(ViewState::Running, 0);
    assert_eq!(running.progress, TFT_PROGRESS_MAX);
    renderer
        .render_block(
            &running,
            TftBackgroundMode::Animated,
            200,
            TFT_ROWS_PER_TRANSFER,
            &mut output,
        )
        .unwrap();
    let progress_offset = 6 * TFT_BYTES_PER_ROW + 22 * 2;
    assert_eq!(&output[progress_offset..progress_offset + 2], &[0x2f, 0x5a]);
}

#[test]
fn progress_endpoints_change_the_composed_bar() {
    let renderer = TftRenderer::new();
    let mut empty = [0_u8; TFT_TRANSFER_BYTES];
    let mut full = [0_u8; TFT_TRANSFER_BYTES];
    renderer
        .render_block(
            &view(ViewState::Paused, 1_500_000),
            TftBackgroundMode::Static,
            200,
            TFT_ROWS_PER_TRANSFER,
            &mut empty,
        )
        .unwrap();
    renderer
        .render_block(
            &view(ViewState::Completed, 0),
            TftBackgroundMode::Static,
            200,
            TFT_ROWS_PER_TRANSFER,
            &mut full,
        )
        .unwrap();
    assert_ne!(empty, full);
}

#[test]
fn static_fallback_is_phase_independent() {
    let mut renderer = TftRenderer::new();
    let mut before = [0_u8; TFT_TRANSFER_BYTES];
    let mut after = [0_u8; TFT_TRANSFER_BYTES];
    let idle = view(ViewState::Idle, 1_500_000);
    renderer
        .render_block(
            &idle,
            TftBackgroundMode::Static,
            0,
            TFT_ROWS_PER_TRANSFER,
            &mut before,
        )
        .unwrap();
    for _ in 0..100 {
        renderer.advance(TftBackgroundMode::Animated);
    }
    renderer
        .render_block(
            &idle,
            TftBackgroundMode::Static,
            0,
            TFT_ROWS_PER_TRANSFER,
            &mut after,
        )
        .unwrap();
    assert_eq!(before, after);
}
