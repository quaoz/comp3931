use std::{
    io::{BufWriter, Write},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::{graphics::gui::DebugInfo, settings::WorldDate};

// ── Rebuild breakdown ───────────────────────────────────────────────────────

#[derive(Default, Clone, Copy)]
pub struct RebuildBreakdown {
    /// L-system expansion and flattening into turtle actions
    pub generate_ms: f32,
    /// Executing actions to create geometry
    pub turtle_ms: f32,
    /// Moving line and mesh geometry out of the turtle into the per-plant cache
    pub extract_ms: f32,
    /// Concatenating every cached plant into the single scene-wide buffer
    pub combine_ms: f32,
    /// Writing the combined buffers to the GPU
    pub upload_ms: f32,
    /// Plants whose geometry was actually rebuilt
    pub plants_built: usize,
}

impl RebuildBreakdown {
    pub fn phases(&self) -> [(&'static str, f32); 5] {
        [
            ("generate", self.generate_ms),
            ("turtle", self.turtle_ms),
            ("extract", self.extract_ms),
            ("combine", self.combine_ms),
            ("upload", self.upload_ms),
        ]
    }

    pub fn total_ms(&self) -> f32 {
        self.phases().iter().map(|&(_, ms)| ms).sum()
    }

    pub fn dominant(&self) -> (&'static str, f32) {
        let total = self.total_ms();
        let (name, ms) = self.phases().into_iter().fold(("—", 0.0), |best, phase| {
            if phase.1 > best.1 { phase } else { best }
        });
        (name, if total > 0.0 { ms / total } else { 0.0 })
    }
}

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
    sum_breakdown: RebuildBreakdown,
    sum_plants_built: u64,
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

    /// Phase timings averaged over every rebuild recorded so far.
    pub fn mean_breakdown(&self) -> RebuildBreakdown {
        if self.rebuild_count == 0 {
            return RebuildBreakdown::default();
        }
        let n = self.rebuild_count as f32;
        RebuildBreakdown {
            generate_ms: self.sum_breakdown.generate_ms / n,
            turtle_ms: self.sum_breakdown.turtle_ms / n,
            extract_ms: self.sum_breakdown.extract_ms / n,
            combine_ms: self.sum_breakdown.combine_ms / n,
            upload_ms: self.sum_breakdown.upload_ms / n,
            plants_built: (self.sum_plants_built / self.rebuild_count) as usize,
        }
    }

    fn ingest(&mut self, frame_ms: f32, rebuild_ms: f32, breakdown: RebuildBreakdown) {
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
            self.sum_breakdown.generate_ms += breakdown.generate_ms;
            self.sum_breakdown.turtle_ms += breakdown.turtle_ms;
            self.sum_breakdown.extract_ms += breakdown.extract_ms;
            self.sum_breakdown.combine_ms += breakdown.combine_ms;
            self.sum_breakdown.upload_ms += breakdown.upload_ms;
            self.sum_plants_built += breakdown.plants_built as u64;
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
/// rebuild_ms, rebuild_full, plants_built,
/// generate_ms, turtle_ms, extract_ms, combine_ms, upload_ms,
/// scene, age, season
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
                     culled,rebuild_ms,rebuild_full,plants_built,generate_ms,turtle_ms,extract_ms,\
                     combine_ms,upload_ms,scene,age,season"
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
        let breakdown = debug_info.scene.breakdown;
        self.summary.ingest(
            debug_info.frame_ms,
            debug_info.scene.last_rebuild_ms,
            breakdown,
        );

        let Some(writer) = self.writer.as_mut() else {
            return;
        };
        let elapsed = self
            .start_time
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        let _ = writeln!(
            writer,
            "{:.3},{:.3},{:.1},{},{},{},{},{},{},{},{},{:.3},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},\
             {},{},{:.4}",
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
            breakdown.plants_built,
            breakdown.generate_ms,
            breakdown.turtle_ms,
            breakdown.extract_ms,
            breakdown.combine_ms,
            breakdown.upload_ms,
            scene_name,
            date.year,
            date.season(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── RebuildBreakdown ──

    #[test]
    fn dominant_picks_the_largest_phase() {
        // Proportions taken from a measured 200-plant rebuild.
        let b = RebuildBreakdown {
            generate_ms: 42.0,
            turtle_ms: 152.0,
            extract_ms: 1.6,
            combine_ms: 83.0,
            upload_ms: 5.0,
            plants_built: 200,
        };
        assert!((b.total_ms() - 283.6).abs() < 1e-3);
        let (name, share) = b.dominant();
        assert_eq!(name, "turtle");
        assert!((share - 152.0 / 283.6).abs() < 1e-4, "share was {share}");
    }

    #[test]
    fn dominant_does_not_divide_by_zero_when_cached() {
        let (_, share) = RebuildBreakdown::default().dominant();
        assert_eq!(share, 0.0);
    }

    #[test]
    fn phases_sum_to_total() {
        let b = RebuildBreakdown {
            generate_ms: 1.0,
            turtle_ms: 2.0,
            extract_ms: 3.0,
            combine_ms: 4.0,
            upload_ms: 5.0,
            plants_built: 1,
        };
        let summed: f32 = b.phases().iter().map(|&(_, ms)| ms).sum();
        assert!((summed - b.total_ms()).abs() < 1e-6);
    }

    // ── PerfSummary ──

    #[test]
    fn mean_breakdown_averages_over_rebuilds_not_frames() {
        let mut s = PerfSummary::default();
        let b = RebuildBreakdown {
            turtle_ms: 10.0,
            combine_ms: 4.0,
            plants_built: 3,
            ..Default::default()
        };
        // Two real rebuilds plus a cached frame. The cached frame contributes no
        // phase time, so including it in the mean would understate every stage.
        s.ingest(16.0, 14.0, b);
        s.ingest(16.0, 14.0, b);
        s.ingest(16.0, 0.0, RebuildBreakdown::default());

        assert_eq!(s.frame_count, 3);
        assert_eq!(s.rebuild_count, 2);

        let mean = s.mean_breakdown();
        assert!((mean.turtle_ms - 10.0).abs() < 1e-4, "{}", mean.turtle_ms);
        assert!((mean.combine_ms - 4.0).abs() < 1e-4, "{}", mean.combine_ms);
        assert_eq!(mean.plants_built, 3);
    }

    #[test]
    fn mean_breakdown_is_zero_before_any_rebuild() {
        let mut s = PerfSummary::default();
        s.ingest(16.0, 0.0, RebuildBreakdown::default());
        assert_eq!(s.mean_breakdown().total_ms(), 0.0);
    }
}
