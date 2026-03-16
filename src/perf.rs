use std::{
    io::{BufWriter, Write},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::{graphics::gui::DebugInfo, settings::WorldDate};

// ── Summary statistics ──────────────────────────────────────────────────────

/// Rolling statistics accumulated over the current (or most recent) recording
/// session
#[derive(Default, Clone, Copy)]
pub struct PerfSummary {
    pub frame_count: u64,
    pub min_frame_ms: f32,
    pub max_frame_ms: f32,
    sum_frame_ms: f64,
    pub rebuild_count: u64,
    sum_rebuild_ms: f64,
}

impl PerfSummary {
    pub fn mean_frame_ms(&self) -> f32 {
        if self.frame_count == 0 {
            0.0
        } else {
            (self.sum_frame_ms / self.frame_count as f64) as f32
        }
    }

    pub fn mean_rebuild_ms(&self) -> f32 {
        if self.rebuild_count == 0 {
            0.0
        } else {
            (self.sum_rebuild_ms / self.rebuild_count as f64) as f32
        }
    }

    fn ingest(&mut self, frame_ms: f32, rebuild_ms: f32) {
        self.frame_count += 1;
        if self.frame_count == 1 {
            self.min_frame_ms = frame_ms;
            self.max_frame_ms = frame_ms;
        } else {
            self.min_frame_ms = self.min_frame_ms.min(frame_ms);
            self.max_frame_ms = self.max_frame_ms.max(frame_ms);
        }
        self.sum_frame_ms += frame_ms as f64;
        if rebuild_ms > 0.0 {
            self.rebuild_count += 1;
            self.sum_rebuild_ms += rebuild_ms as f64;
        }
    }
}

// ── Logger ──────────────────────────────────────────────────────────────────

/// Records per-frame performance data to a CSV file on disk and maintains
/// rolling summary statistics for the in-app debug HUD.
///
/// CSV columns (one row per rendered frame):
/// ```text
/// elapsed_s, frame_ms, fps, mesh_tris, line_verts,
/// plant_count, near, mid, far, beyond, culled,
/// rebuild_ms, rebuild_full, scene, age, season
/// ```
#[derive(Default)]
pub struct PerfLogger {
    recording: bool,
    writer: Option<BufWriter<std::fs::File>>,
    start_time: Option<Instant>,
    /// Rolling stats for the current (or most recently completed) session.
    pub summary: PerfSummary,
    /// Path of the file currently (or most recently) being written to.
    pub current_file: Option<String>,
}

impl PerfLogger {
    /// Open a new timestamped CSV file and begin recording
    pub fn start(&mut self) -> Option<String> {
        if self.recording {
            return self.current_file.clone();
        }
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = format!("perf_{ts}.csv");
        match std::fs::File::create(&path) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                let _ = writeln!(
                    writer,
                    "elapsed_s,frame_ms,fps,mesh_tris,line_verts,plant_count,near,mid,far,beyond,\
                     culled,rebuild_ms,rebuild_full,scene,age,season"
                );
                self.writer = Some(writer);
                self.start_time = Some(Instant::now());
                self.summary = PerfSummary::default();
                self.recording = true;
                self.current_file = Some(path.clone());
                Some(path)
            }
            Err(_) => None,
        }
    }

    /// Flush and close the current file, stopping recording
    pub fn stop(&mut self) {
        if let Some(mut w) = self.writer.take() {
            let _ = w.flush();
        }
        self.recording = false;
        self.start_time = None;
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Seconds elapsed since recording started
    pub fn elapsed_secs(&self) -> f32 {
        self.start_time
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0)
    }

    /// Record one rendered frame
    pub fn record(&mut self, debug_info: &DebugInfo, scene_name: &str, date: WorldDate) {
        self.summary
            .ingest(debug_info.frame_ms, debug_info.scene.last_rebuild_ms);

        let Some(writer) = self.writer.as_mut() else {
            return;
        };
        let elapsed = self
            .start_time
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        let _ = writeln!(
            writer,
            "{:.3},{:.3},{:.1},{},{},{},{},{},{},{},{},{:.3},{},{},{},{:.4}",
            elapsed,
            debug_info.frame_ms,
            debug_info.fps,
            debug_info.mesh_index_count / 3,
            debug_info.scene.line_vertex_count,
            debug_info.scene.plant_count,
            debug_info.scene.lod_tier_counts[0],
            debug_info.scene.lod_tier_counts[1],
            debug_info.scene.lod_tier_counts[2],
            debug_info.scene.lod_tier_counts[3],
            debug_info.scene.culled_count,
            debug_info.scene.last_rebuild_ms,
            debug_info.scene.last_rebuild_full as u8,
            scene_name,
            date.year,
            date.season(),
        );
    }
}
