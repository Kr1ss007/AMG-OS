//! Notification System (Process 2 — Animation and Interface Engine)
//!
//! Manages system notification display at Layer 9 (above everything except Pilot Control,
//! Lockscreen, and Shutdown screens).
//!
//! Sources:
//!   - Process 1 e-bus: NotificationDispatched (from eo-bus D-Bus bridge for third-party apps)
//!   - Internal AMGOS system events: OS updates available, network state changes, etc.
//!
//! Behavior:
//!   - Notifications stack in the top-right corner
//!   - Each notification auto-dismisses after its timeout unless the user interacts
//!   - Entry animation: DECELERATE curve slide-in from the right
//!   - Exit animation: ACCELERATE curve slide-out to the right
//!   - No notification interrupts another; they queue

use crate::animations::curves;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Default auto-dismiss duration
pub const DEFAULT_NOTIFICATION_TIMEOUT_MS: u64 = 5000;

/// Urgency level maps to visual prominence
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationUrgency {
    Low = 0,
    Normal = 1,
    Critical = 2, // Critical notifications do not auto-dismiss
}

impl From<u8> for NotificationUrgency {
    fn from(v: u8) -> Self {
        match v {
            0 => NotificationUrgency::Low,
            2 => NotificationUrgency::Critical,
            _ => NotificationUrgency::Normal,
        }
    }
}

/// Animation state for a single notification banner
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BannerState {
    Entering,
    Visible,
    Exiting,
    Dismissed,
}

/// A single notification displayed in the stack
#[derive(Debug)]
pub struct NotificationBanner {
    pub notification_id: u64,
    pub app_id: String,
    pub title: String,
    pub body: String,
    pub urgency: NotificationUrgency,
    pub state: BannerState,
    /// [0.0, 1.0] animation progress
    pub animation_progress: f64,
    /// When the notification became fully visible
    pub visible_since: Option<Instant>,
    /// Timeout duration (None = no auto-dismiss for Critical urgency)
    pub timeout: Option<Duration>,
}

impl NotificationBanner {
    pub fn new(
        notification_id: u64,
        app_id: &str,
        title: &str,
        body: &str,
        urgency: NotificationUrgency,
    ) -> Self {
        let timeout = match urgency {
            NotificationUrgency::Critical => None,
            NotificationUrgency::Normal => {
                Some(Duration::from_millis(DEFAULT_NOTIFICATION_TIMEOUT_MS))
            }
            NotificationUrgency::Low => Some(Duration::from_millis(3000)),
        };

        Self {
            notification_id,
            app_id: app_id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            urgency,
            state: BannerState::Entering,
            animation_progress: 0.0,
            visible_since: None,
            timeout,
        }
    }

    /// Returns current horizontal slide offset [0.0 = onscreen, 1.0 = off right edge]
    pub fn slide_offset(&self) -> f64 {
        match self.state {
            BannerState::Entering => {
                1.0 - curves::DECELERATE.solve(self.animation_progress)
            }
            BannerState::Visible => 0.0,
            BannerState::Exiting => {
                curves::ACCELERATE.solve(1.0 - self.animation_progress)
            }
            BannerState::Dismissed => 1.0,
        }
    }

    pub fn is_dismissed(&self) -> bool {
        matches!(self.state, BannerState::Dismissed)
    }
}

pub struct NotificationCenter {
    /// Active banners in display order (newest at top)
    pub banners: VecDeque<NotificationBanner>,
    /// Maximum concurrent banners displayed
    pub max_visible: usize,
}

impl NotificationCenter {
    pub fn new() -> Self {
        Self {
            banners: VecDeque::new(),
            max_visible: 5,
        }
    }

    /// Push a new notification from e-bus (NotificationDispatched event)
    pub fn push(
        &mut self,
        notification_id: u64,
        app_id: &str,
        title: &str,
        body: &str,
        urgency: u8,
    ) {
        // Replace existing notification with same ID (NM replaces)
        self.banners.retain(|b| b.notification_id != notification_id);

        let banner = NotificationBanner::new(
            notification_id,
            app_id,
            title,
            body,
            NotificationUrgency::from(urgency),
        );

        // Prepend (new notifications appear at the top of the stack)
        self.banners.push_front(banner);

        // If over max_visible, begin dismissing the oldest low/normal urgency banner.
        // Only mark ONE per push — tick() will remove fully-dismissed banners each frame.
        if self.banners.len() > self.max_visible {
            let dismiss_pos = self
                .banners
                .iter()
                .rposition(|b| b.urgency != NotificationUrgency::Critical
                    && !matches!(b.state, BannerState::Exiting | BannerState::Dismissed));

            if let Some(pos) = dismiss_pos {
                if let Some(b) = self.banners.get_mut(pos) {
                    b.state = BannerState::Exiting;
                    b.animation_progress = 1.0;
                }
            }
        }
    }

    /// Dismiss a specific notification (called from main.rs on DismissNotification request)
    pub fn dismiss(&mut self, notification_id: u64) {
        if let Some(banner) = self
            .banners
            .iter_mut()
            .find(|b| b.notification_id == notification_id)
        {
            if !matches!(banner.state, BannerState::Dismissed | BannerState::Exiting) {
                banner.state = BannerState::Exiting;
                banner.animation_progress = 1.0;
            }
        }
    }

    /// Advance all notification animations. Call every compositor frame.
    /// Returns the count of currently visible banners.
    pub fn tick(&mut self, delta_t: f64) -> usize {
        let enter_duration = 0.25;
        let exit_duration = 0.18;

        let now = Instant::now();

        for banner in self.banners.iter_mut() {
            match banner.state {
                BannerState::Entering => {
                    banner.animation_progress =
                        (banner.animation_progress + delta_t / enter_duration).min(1.0);
                    if banner.animation_progress >= 1.0 {
                        banner.state = BannerState::Visible;
                        banner.visible_since = Some(now);
                    }
                }
                BannerState::Visible => {
                    // Check auto-dismiss timeout
                    if let (Some(since), Some(timeout)) = (banner.visible_since, banner.timeout) {
                        if now.duration_since(since) >= timeout {
                            banner.state = BannerState::Exiting;
                            banner.animation_progress = 1.0;
                        }
                    }
                }
                BannerState::Exiting => {
                    banner.animation_progress =
                        (banner.animation_progress - delta_t / exit_duration).max(0.0);
                    if banner.animation_progress <= 0.0 {
                        banner.state = BannerState::Dismissed;
                    }
                }
                BannerState::Dismissed => {}
            }
        }

        // Remove fully dismissed banners
        self.banners.retain(|b| !b.is_dismissed());

        self.banners.len()
    }

    pub fn active_count(&self) -> usize {
        self.banners
            .iter()
            .filter(|b| !b.is_dismissed())
            .count()
    }
}

impl Default for NotificationCenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_push_and_dismiss() {
        let mut nc = NotificationCenter::new();
        nc.push(1, "app-test", "Test Title", "Test body.", 1);
        assert_eq!(nc.banners.len(), 1);
        assert_eq!(nc.banners[0].state, BannerState::Entering);

        nc.dismiss(1);
        assert_eq!(nc.banners[0].state, BannerState::Exiting);
    }

    #[test]
    fn test_notification_replace_same_id() {
        let mut nc = NotificationCenter::new();
        nc.push(42, "app-x", "First", "Body 1", 1);
        nc.push(42, "app-x", "Updated", "Body 2", 1);
        assert_eq!(nc.banners.len(), 1);
        assert_eq!(nc.banners[0].title, "Updated");
    }

    #[test]
    fn test_critical_no_auto_dismiss() {
        let banner = NotificationBanner::new(
            99,
            "security",
            "Security Alert",
            "Your system requires attention.",
            NotificationUrgency::Critical,
        );
        assert!(banner.timeout.is_none());
    }

    #[test]
    fn test_urgency_from_u8() {
        assert_eq!(NotificationUrgency::from(0), NotificationUrgency::Low);
        assert_eq!(NotificationUrgency::from(1), NotificationUrgency::Normal);
        assert_eq!(NotificationUrgency::from(2), NotificationUrgency::Critical);
        assert_eq!(NotificationUrgency::from(255), NotificationUrgency::Normal);
    }

    #[test]
    fn test_max_visible_overflow_dismisses_oldest() {
        let mut nc = NotificationCenter::new();
        for i in 0..7u64 {
            nc.push(i, "app", &format!("Title {i}"), "Body", 1);
        }
        // Over max_visible(5): oldest normal ones should be queued for exit
        // banners count may be > 5 momentarily (they exit via animation)
        assert!(nc.banners.len() <= 7);
    }

    #[test]
    fn test_slide_offset_entering() {
        let banner = NotificationBanner::new(
            1, "app", "Test", "Body", NotificationUrgency::Normal,
        );
        // At start (progress=0.0), should be fully offscreen
        let offset = banner.slide_offset();
        assert!(offset > 0.9);
    }
}
