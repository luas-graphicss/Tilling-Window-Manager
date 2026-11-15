use std::collections::HashSet;
use std::time::{Duration, Instant};

use smithay::desktop::PopupManager;
use smithay::input::keyboard::{Keycode, XkbConfig};
use smithay::input::{Seat, SeatState};
use smithay::reexports::calloop::{LoopHandle, LoopSignal};
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::wayland::compositor::CompositorState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shell::xdg::decoration::XdgDecorationState;
use smithay::wayland::shm::ShmState;
use tilling_config::{ConfigSource, DEFAULT_MODE, ValidatedConfig};
use tilling_core::DesktopState;

use super::{OutputRegistry, Runtime};
use crate::control::ControlPeers;
use crate::desktop::initial_desktop;
use crate::error::CompositorError;
use crate::input::{BindingTable, InputSerials, Launcher};
use crate::scene::Scene;
use crate::shell::WindowRegistry;
use crate::transaction::Coordinator;

const SEAT_NAME: &str = "seat0";
const EVENT_TIME_WRAP: u128 = 1 << 32;

pub(crate) struct Settings {
    pub(crate) config: ValidatedConfig,
    pub(crate) config_source: ConfigSource,
    pub(crate) transaction_timeout: Duration,
    pub(crate) launcher: Launcher,
}

pub(crate) struct Compositor {
    pub(crate) display_handle: DisplayHandle,
    pub(crate) loop_handle: LoopHandle<'static, Runtime>,
    loop_signal: LoopSignal,
    started: Instant,
    pub(crate) compositor_state: CompositorState,
    pub(crate) shm_state: ShmState,
    pub(crate) xdg_shell_state: XdgShellState,
    pub(crate) seat_state: SeatState<Self>,
    pub(crate) seat: Seat<Self>,
    pub(crate) popups: PopupManager,
    pub(crate) config: ValidatedConfig,
    pub(crate) config_source: ConfigSource,
    pub(crate) config_revision: u64,
    pub(crate) bindings: BindingTable,
    pub(crate) keyboard_mode: String,
    pub(crate) suppressed_keys: HashSet<Keycode>,
    pub(crate) input_serials: InputSerials,
    pub(crate) launcher: Launcher,
    pub(crate) desktop: DesktopState,
    pub(crate) windows: WindowRegistry,
    pub(crate) outputs: OutputRegistry,
    pub(crate) scene: Scene,
    pub(crate) transactions: Coordinator,
    pub(crate) control: ControlPeers,
    frame_demanded: bool,
}

impl Compositor {
    pub(crate) fn new(
        display_handle: DisplayHandle,
        loop_handle: LoopHandle<'static, Runtime>,
        loop_signal: LoopSignal,
        settings: Settings,
    ) -> Result<Self, CompositorError> {
        let Settings {
            config,
            config_source,
            transaction_timeout,
            launcher,
        } = settings;
        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, Vec::new());
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        XdgDecorationState::new::<Self>(&display_handle);
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&display_handle, SEAT_NAME);
        let keyboard = &config.input().keyboard;
        seat.add_keyboard(
            XkbConfig {
                layout: &keyboard.layout,
                variant: &keyboard.variant,
                ..XkbConfig::default()
            },
            i32::try_from(keyboard.repeat_delay_ms).unwrap_or(i32::MAX),
            i32::try_from(keyboard.repeat_rate).unwrap_or(i32::MAX),
        )
        .map_err(CompositorError::Keyboard)?;
        seat.add_pointer();
        let desktop = initial_desktop(&config)?;
        let bindings = BindingTable::compile(config.bindings());
        Ok(Self {
            display_handle,
            loop_handle,
            loop_signal,
            started: Instant::now(),
            compositor_state,
            shm_state,
            xdg_shell_state,
            seat_state,
            seat,
            popups: PopupManager::default(),
            config,
            config_source,
            config_revision: 0,
            bindings,
            keyboard_mode: DEFAULT_MODE.to_owned(),
            suppressed_keys: HashSet::new(),
            input_serials: InputSerials::default(),
            launcher,
            desktop,
            windows: WindowRegistry::default(),
            outputs: OutputRegistry::default(),
            scene: Scene::default(),
            transactions: Coordinator::new(transaction_timeout),
            control: ControlPeers::default(),
            frame_demanded: false,
        })
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    pub(crate) fn event_time(&self) -> u32 {
        u32::try_from(self.elapsed().as_millis() % EVENT_TIME_WRAP).unwrap_or_default()
    }

    pub(crate) fn stop(&self) {
        self.loop_signal.stop();
        self.loop_signal.wakeup();
    }

    pub(crate) fn demand_frame(&mut self) {
        self.frame_demanded = true;
    }

    pub(crate) fn take_frame_demand(&mut self) -> bool {
        std::mem::take(&mut self.frame_demanded)
    }
}
