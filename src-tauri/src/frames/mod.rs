//! デバイスフレーム合成機能。カタログ（メタデータ）・保存場所・取り込み・合成器に分かれる。
//! 設計: docs/superpowers/specs/2026-08-30-device-frame-design.md

pub mod compose;
pub mod catalog;
pub mod store;
pub mod import;

use serde::{Deserialize, Serialize};

/// 画像内の矩形（左上原点、ピクセル単位）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
}
