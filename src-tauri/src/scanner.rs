use std::sync::Mutex;
use xcap::Monitor;
use image::{RgbaImage, Rgba, ImageBuffer};

/// Cached button positions for 36-key mode
/// Each position is (x, y) screen coordinates for clicking
#[derive(Debug, Clone, Default)]
pub struct ButtonPositions {
    // Sharp keys (9 keys) - click positions: C#, F#, G# for each octave (low, mid, high)
    pub sharps: Vec<(i32, i32)>,
    // Flat keys (6 keys) - click positions: Eb, Bb for each octave (low, mid, high)
    pub flats: Vec<(i32, i32)>,
    pub is_cached: bool,
}

lazy_static::lazy_static! {
    pub static ref BUTTON_CACHE: Mutex<ButtonPositions> = Mutex::new(ButtonPositions::default());
}

/// Scan the screen to detect button positions
/// Returns true if detection was successful
pub fn scan_button_positions() -> Result<bool, String> {
    // Capture the primary monitor
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    let monitor = monitors.first().ok_or("No monitor found")?;

    let screenshot = monitor.capture_image().map_err(|e| e.to_string())?;
    let width = screenshot.width();
    let height = screenshot.height();

    println!("Screenshot captured: {}x{}", width, height);

    // Detect buttons and save debug image
    let detected = detect_button_grid(&screenshot)?;

    if detected.is_cached {
        let mut cache = BUTTON_CACHE.lock().unwrap();
        *cache = detected;
        println!("Button positions cached successfully");
        println!("Sharps ({}):", cache.sharps.len());
        for (i, pos) in cache.sharps.iter().enumerate() {
            println!("  [{}] {:?}", i, pos);
        }
        println!("Flats ({}):", cache.flats.len());
        for (i, pos) in cache.flats.iter().enumerate() {
            println!("  [{}] {:?}", i, pos);
        }
        Ok(true)
    } else {
        Err("Could not detect button positions".to_string())
    }
}

/// Save debug image with detected buttons marked
fn save_debug_image(img: &RgbaImage, buttons: &[(i32, i32)], sharps: &[(i32, i32)], flats: &[(i32, i32)]) {
    let mut debug_img = img.clone();

    // Draw all detected buttons in blue
    for &(x, y) in buttons {
        draw_circle(&mut debug_img, x, y, 25, [0, 0, 255, 255]);
    }

    // Draw sharps in green
    for &(x, y) in sharps {
        draw_circle(&mut debug_img, x, y, 30, [0, 255, 0, 255]);
        draw_cross(&mut debug_img, x, y, 20, [0, 255, 0, 255]);
    }

    // Draw flats in red
    for &(x, y) in flats {
        draw_circle(&mut debug_img, x, y, 30, [255, 0, 0, 255]);
        draw_cross(&mut debug_img, x, y, 20, [255, 0, 0, 255]);
    }

    // Save to exe directory
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let debug_path = exe_dir.join("debug_screenshot.png");
            if let Err(e) = debug_img.save(&debug_path) {
                println!("Failed to save debug image: {}", e);
            } else {
                println!("Debug image saved to: {:?}", debug_path);
            }
        }
    }
}

fn draw_circle(img: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: [u8; 4]) {
    let width = img.width() as i32;
    let height = img.height() as i32;

    for angle in 0..360 {
        let rad = (angle as f32) * std::f32::consts::PI / 180.0;
        let x = cx + (radius as f32 * rad.cos()) as i32;
        let y = cy + (radius as f32 * rad.sin()) as i32;

        if x >= 0 && x < width && y >= 0 && y < height {
            img.put_pixel(x as u32, y as u32, Rgba(color));
            // Make it thicker
            if x + 1 < width { img.put_pixel((x + 1) as u32, y as u32, Rgba(color)); }
            if y + 1 < height { img.put_pixel(x as u32, (y + 1) as u32, Rgba(color)); }
        }
    }
}

fn draw_cross(img: &mut RgbaImage, cx: i32, cy: i32, size: i32, color: [u8; 4]) {
    let width = img.width() as i32;
    let height = img.height() as i32;

    for i in -size..=size {
        let x = cx + i;
        let y = cy;
        if x >= 0 && x < width && y >= 0 && y < height {
            img.put_pixel(x as u32, y as u32, Rgba(color));
        }
        let x = cx;
        let y = cy + i;
        if x >= 0 && x < width && y >= 0 && y < height {
            img.put_pixel(x as u32, y as u32, Rgba(color));
        }
    }
}

/// Detect the button grid from the screenshot
fn detect_button_grid(img: &RgbaImage) -> Result<ButtonPositions, String> {
    let width = img.width() as i32;
    let height = img.height() as i32;

    // Look for circular buttons with specific characteristics
    // The game buttons are semi-transparent dark circles

    let mut button_centers: Vec<(i32, i32)> = Vec::new();

    // Adjust detection parameters based on resolution
    let scale = (width as f32 / 1920.0).max(1.0);
    let button_radius = (40.0 * scale) as i32;
    let step = (15.0 * scale) as i32;
    let min_spacing = (button_radius as f32 * 1.5) as i32;

    println!("Detection params: scale={:.2}, radius={}, step={}", scale, button_radius, step);

    // Scan the lower portion of the screen where the instrument UI typically is
    let scan_top = height / 2;  // Start from middle of screen
    let scan_bottom = height - 50;
    let scan_left = 50;
    let scan_right = width - 50;

    for y in (scan_top..scan_bottom).step_by(step as usize) {
        for x in (scan_left..scan_right).step_by(step as usize) {
            if is_game_button(img, x, y, button_radius) {
                // Refine position to find actual button center
                let (cx, cy) = refine_button_center(img, x, y, button_radius);

                // Check if we already have a nearby center (avoid duplicates)
                let dominated = button_centers.iter().any(|(ex, ey)| {
                    let dx = (ex - cx).abs();
                    let dy = (ey - cy).abs();
                    dx < min_spacing && dy < min_spacing
                });

                if !dominated {
                    button_centers.push((cx, cy));
                }
            }
        }
    }

    println!("Found {} potential button centers", button_centers.len());

    // Sort by Y first, then by X
    button_centers.sort_by(|a, b| {
        match a.1.cmp(&b.1) {
            std::cmp::Ordering::Equal => a.0.cmp(&b.0),
            other => other,
        }
    });

    // Group into rows based on Y coordinate
    let row_threshold = (50.0 * scale) as i32;
    let mut rows: Vec<Vec<(i32, i32)>> = Vec::new();
    let mut current_row: Vec<(i32, i32)> = Vec::new();
    let mut last_y = -1000;

    for (x, y) in &button_centers {
        if (y - last_y).abs() > row_threshold {
            if !current_row.is_empty() {
                current_row.sort_by_key(|(x, _)| *x);
                rows.push(current_row);
                current_row = Vec::new();
            }
        }
        current_row.push((*x, *y));
        last_y = *y;
    }
    if !current_row.is_empty() {
        current_row.sort_by_key(|(x, _)| *x);
        rows.push(current_row);
    }

    println!("Grouped into {} rows", rows.len());
    for (i, row) in rows.iter().enumerate() {
        println!("  Row {}: {} buttons, Y~{}", i, row.len(),
            if row.is_empty() { 0 } else { row[0].1 });
    }

    // Find the 3 main instrument rows (should have 12 buttons each for 36-key mode)
    // Filter rows that have roughly the right number of buttons (10-14)
    let instrument_rows: Vec<&Vec<(i32, i32)>> = rows.iter()
        .filter(|row| row.len() >= 10 && row.len() <= 14)
        .collect();

    println!("Found {} instrument rows (10-14 buttons each)", instrument_rows.len());

    let positions = if instrument_rows.len() >= 3 {
        // Use the detected rows to identify sharp/flat positions
        identify_positions_from_rows(&instrument_rows)
    } else {
        // Fallback: use heuristic based on screen position
        println!("Using fallback detection");
        estimate_positions_fallback(width, height, scale)
    };

    // Save debug image
    save_debug_image(img, &button_centers, &positions.sharps, &positions.flats);

    Ok(positions)
}

/// Refine button center by searching for the darkest/most consistent area nearby
fn refine_button_center(img: &RgbaImage, initial_x: i32, initial_y: i32, radius: i32) -> (i32, i32) {
    let width = img.width() as i32;
    let height = img.height() as i32;
    let search_range = radius / 2;

    let mut best_x = initial_x;
    let mut best_y = initial_y;
    let mut best_score = f32::MAX;

    // Search in a small area around the initial detection
    for dy in -search_range..=search_range {
        for dx in -search_range..=search_range {
            let x = initial_x + dx;
            let y = initial_y + dy;

            if x < radius || x >= width - radius || y < radius || y >= height - radius {
                continue;
            }

            // Calculate score based on how "button-like" this center is
            // Lower score = better center (darker, more consistent)
            let score = calculate_center_score(img, x, y, radius);
            if score < best_score {
                best_score = score;
                best_x = x;
                best_y = y;
            }
        }
    }

    (best_x, best_y)
}

/// Calculate how good a position is as a button center (lower = better)
fn calculate_center_score(img: &RgbaImage, cx: i32, cy: i32, radius: i32) -> f32 {
    let mut total_brightness = 0.0;
    let mut variance = 0.0;
    let samples = 8;
    let mut values = Vec::new();

    // Sample points in a small circle around center
    let check_radius = radius / 3;
    for i in 0..samples {
        let angle = (i as f32) * 2.0 * std::f32::consts::PI / (samples as f32);
        let x = cx + (check_radius as f32 * angle.cos()) as i32;
        let y = cy + (check_radius as f32 * angle.sin()) as i32;

        let p = img.get_pixel(x as u32, y as u32);
        let b = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
        values.push(b);
        total_brightness += b;
    }

    // Also sample the center
    let center = img.get_pixel(cx as u32, cy as u32);
    let center_b = (center[0] as f32 + center[1] as f32 + center[2] as f32) / 3.0;
    values.push(center_b);
    total_brightness += center_b;

    let mean = total_brightness / (samples + 1) as f32;

    // Calculate variance (consistency)
    for v in &values {
        variance += (v - mean).powi(2);
    }
    variance /= values.len() as f32;

    // Score: prefer darker and more consistent areas
    // Lower brightness + lower variance = better
    mean + variance.sqrt() * 0.5
}

/// Check if a position looks like a game button
fn is_game_button(img: &RgbaImage, cx: i32, cy: i32, radius: i32) -> bool {
    let width = img.width() as i32;
    let height = img.height() as i32;

    // Check center pixel
    if cx < radius || cx >= width - radius || cy < radius || cy >= height - radius {
        return false;
    }

    let center = img.get_pixel(cx as u32, cy as u32);

    // Game buttons appear to be semi-transparent dark circles
    // Check for dark/medium gray color
    let r = center[0] as f32;
    let g = center[1] as f32;
    let b = center[2] as f32;
    let brightness = (r + g + b) / 3.0;

    // Button centers should be darker (roughly 30-100 brightness)
    if brightness < 25.0 || brightness > 130.0 {
        return false;
    }

    // Check if it's roughly circular by sampling points around the center
    let check_radius = radius / 2;
    let mut dark_count = 0;
    let mut edge_dark_count = 0;
    let samples = 16;

    // Check inner area (should be mostly dark)
    for i in 0..samples {
        let angle = (i as f32) * 2.0 * std::f32::consts::PI / (samples as f32);
        let x = cx + (check_radius as f32 * 0.5 * angle.cos()) as i32;
        let y = cy + (check_radius as f32 * 0.5 * angle.sin()) as i32;

        if x >= 0 && x < width && y >= 0 && y < height {
            let p = img.get_pixel(x as u32, y as u32);
            let b = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
            if b >= 25.0 && b <= 140.0 {
                dark_count += 1;
            }
        }
    }

    // Check edge area (should transition to lighter/different)
    for i in 0..samples {
        let angle = (i as f32) * 2.0 * std::f32::consts::PI / (samples as f32);
        let x = cx + (radius as f32 * 1.2 * angle.cos()) as i32;
        let y = cy + (radius as f32 * 1.2 * angle.sin()) as i32;

        if x >= 0 && x < width && y >= 0 && y < height {
            let p = img.get_pixel(x as u32, y as u32);
            let b = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
            // Edge should be different from center
            if (b - brightness).abs() > 20.0 {
                edge_dark_count += 1;
            }
        }
    }

    // Should have mostly dark inner pixels and some edge contrast
    dark_count >= samples * 3 / 4 && edge_dark_count >= samples / 4
}

/// Identify sharp and flat positions from detected rows
fn identify_positions_from_rows(rows: &[&Vec<(i32, i32)>]) -> ButtonPositions {
    let mut positions = ButtonPositions::default();

    // Take the 3 rows closest to bottom (instrument rows)
    // Reverse order: bottom row = low octave, middle = mid, top = high
    let mut sorted_rows: Vec<&Vec<(i32, i32)>> = rows.iter().cloned().collect();
    sorted_rows.sort_by(|a, b| b[0].1.cmp(&a[0].1)); // Sort by Y descending (bottom first)

    // The layout from the image shows 12 buttons per row:
    // Index: 0    1    2    3    4    5    6    7    8    9   10   11
    // Note:  C   C#    D   Eb    E    F   F#    G   G#    A   Bb    B
    // Type:  N    S    N    F    N    N    S    N    S    N    F    N
    // Where N=natural, S=sharp, F=flat

    for (octave_idx, row) in sorted_rows.iter().take(3).enumerate() {
        println!("Processing octave {} with {} buttons", octave_idx, row.len());

        if row.len() >= 12 {
            // Full 12-button row
            // Sharps at indices 1, 6, 8 (C#, F#, G#)
            positions.sharps.push(row[1]);
            positions.sharps.push(row[6]);
            positions.sharps.push(row[8]);

            // Flats at indices 3, 10 (Eb, Bb)
            positions.flats.push(row[3]);
            positions.flats.push(row[10]);
        } else if row.len() >= 7 {
            // Only natural keys detected, estimate sharp/flat positions
            // Natural keys at indices 0-6 (C, D, E, F, G, A, B)
            let spacing = if row.len() > 1 { row[1].0 - row[0].0 } else { 80 };

            // C# between C(0) and D(1)
            let cs = ((row[0].0 + row[1].0) / 2, row[0].1);
            positions.sharps.push(cs);

            // F# between F(3) and G(4)
            if row.len() > 4 {
                let fs = ((row[3].0 + row[4].0) / 2, row[3].1);
                positions.sharps.push(fs);
            }

            // G# between G(4) and A(5)
            if row.len() > 5 {
                let gs = ((row[4].0 + row[5].0) / 2, row[4].1);
                positions.sharps.push(gs);
            }

            // Eb - use E position (index 2)
            if row.len() > 2 {
                positions.flats.push(row[2]);
            }

            // Bb - use B position (index 6)
            if row.len() > 6 {
                positions.flats.push(row[6]);
            }
        }
    }

    positions.is_cached = positions.sharps.len() >= 3;
    positions
}

/// Fallback: estimate button positions based on typical UI layout
fn estimate_positions_fallback(width: i32, height: i32, scale: f32) -> ButtonPositions {
    let mut positions = ButtonPositions::default();

    // Typical instrument UI layout:
    // - Located in the lower third of the screen
    // - 3 rows of buttons
    // - Each row has 12 buttons for 36-key mode

    let ui_bottom = height - (height / 10);
    let ui_top = height - (height / 3);
    let ui_left = width / 5;
    let ui_right = width - (width / 5);

    let row_height = (ui_bottom - ui_top) / 3;
    let button_spacing = (ui_right - ui_left) / 12;

    // For each octave (bottom=low, middle=mid, top=high)
    for octave in 0..3 {
        // Y position: bottom row first (low), then mid, then high
        let y = ui_bottom - row_height / 2 - (octave * row_height);

        // Sharps at columns 1, 6, 8 (C#, F#, G#)
        for &col in &[1, 6, 8] {
            let x = ui_left + button_spacing / 2 + col * button_spacing;
            positions.sharps.push((x, y));
        }

        // Flats at columns 3, 10 (Eb, Bb)
        for &col in &[3, 10] {
            let x = ui_left + button_spacing / 2 + col * button_spacing;
            positions.flats.push((x, y));
        }
    }

    positions.is_cached = true;
    positions
}

/// Get cached button positions (returns None if not cached)
pub fn get_cached_positions() -> Option<ButtonPositions> {
    let cache = BUTTON_CACHE.lock().unwrap();
    if cache.is_cached {
        Some(cache.clone())
    } else {
        None
    }
}

/// Clear the button position cache
pub fn clear_cache() {
    let mut cache = BUTTON_CACHE.lock().unwrap();
    *cache = ButtonPositions::default();
}
