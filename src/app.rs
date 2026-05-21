use std::collections::VecDeque;
use crate::wslc::types::*;

const MAX_STATS_HISTORY: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceSection {
    Containers,
    Images,
    Volumes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Main, // Logs + Stats combined (default for containers)
    Info,
    Env,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    ResourceList,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
    Confirm,
    ActionMenu,
    PullInput,
}

#[derive(Debug, Clone)]
pub struct ActionMenuItem {
    pub label: String,
    pub hotkey: char,
}

#[derive(Debug, Clone, Default)]
pub struct StatsHistory {
    pub cpu: VecDeque<f64>,
    pub memory: VecDeque<f64>,
}

impl StatsHistory {
    pub fn push_cpu(&mut self, val: f64) {
        if self.cpu.len() >= MAX_STATS_HISTORY {
            self.cpu.pop_front();
        }
        self.cpu.push_back(val);
    }

    pub fn push_memory(&mut self, val: f64) {
        if self.memory.len() >= MAX_STATS_HISTORY {
            self.memory.pop_front();
        }
        self.memory.push_back(val);
    }
}

pub struct App {
    pub running: bool,

    // Data
    pub containers: Vec<Container>,
    pub images: Vec<Image>,
    pub volumes: Vec<Volume>,

    // Navigation
    pub active_section: ResourceSection,
    pub focus: FocusPanel,
    pub container_index: usize,
    pub image_index: usize,
    pub volume_index: usize,
    pub detail_tab: DetailTab,

    // Section collapse state
    pub containers_collapsed: bool,
    pub images_collapsed: bool,
    pub volumes_collapsed: bool,

    // Detail data
    pub inspect_text: String,
    pub logs_text: String,
    pub stats_text: String,
    pub stats_history: std::collections::HashMap<String, StatsHistory>,
    pub current_stats: Option<Stats>,
    pub logs_scroll: u16,

    // Input
    pub input_mode: InputMode,
    pub filter_text: String,
    pub pull_input: String,

    // Action menu
    pub action_menu_items: Vec<ActionMenuItem>,
    pub action_menu_index: usize,

    // Confirm dialog
    pub confirm_message: String,
    pub confirm_action: Option<ConfirmAction>,

    // Flash message
    pub flash_message: Option<String>,
    pub flash_timer: u8,

    // Splash header
    pub show_splash: bool,
    pub splash_ticks: u16,

    // Loading
    pub loading: bool,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    RemoveContainer(String),
    RemoveImage(String),
    RemoveVolume(String),
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            containers: Vec::new(),
            images: Vec::new(),
            volumes: Vec::new(),
            active_section: ResourceSection::Containers,
            focus: FocusPanel::ResourceList,
            container_index: 0,
            image_index: 0,
            volume_index: 0,
            detail_tab: DetailTab::Main,
            containers_collapsed: false,
            images_collapsed: false,
            volumes_collapsed: false,
            inspect_text: String::new(),
            logs_text: String::new(),
            stats_text: String::new(),
            stats_history: std::collections::HashMap::new(),
            current_stats: None,
            logs_scroll: 0,
            input_mode: InputMode::Normal,
            filter_text: String::new(),
            pull_input: String::new(),
            action_menu_items: Vec::new(),
            action_menu_index: 0,
            confirm_message: String::new(),
            confirm_action: None,
            flash_message: None,
            flash_timer: 0,
            show_splash: true,
            splash_ticks: 0,
            loading: false,
        }
    }

    pub fn selected_container(&self) -> Option<&Container> {
        let filtered = self.filtered_containers();
        filtered.into_iter().nth(self.container_index)
    }

    pub fn selected_image(&self) -> Option<&Image> {
        let filtered = self.filtered_images();
        filtered.into_iter().nth(self.image_index)
    }

    pub fn selected_volume(&self) -> Option<&Volume> {
        let filtered = self.filtered_volumes();
        filtered.into_iter().nth(self.volume_index)
    }

    pub fn selected_resource_id(&self) -> Option<String> {
        match self.active_section {
            ResourceSection::Containers => self.selected_container().map(|c| c.id.clone()),
            ResourceSection::Images => self.selected_image().map(|i| i.id.clone()),
            ResourceSection::Volumes => self.selected_volume().map(|v| v.name.clone()),
        }
    }

    pub fn filtered_containers(&self) -> Vec<&Container> {
        if self.filter_text.is_empty() {
            self.containers.iter().collect()
        } else {
            let f = self.filter_text.to_lowercase();
            self.containers
                .iter()
                .filter(|c| c.name.to_lowercase().contains(&f) || c.image.to_lowercase().contains(&f))
                .collect()
        }
    }

    pub fn filtered_images(&self) -> Vec<&Image> {
        if self.filter_text.is_empty() {
            self.images.iter().collect()
        } else {
            let f = self.filter_text.to_lowercase();
            self.images
                .iter()
                .filter(|i| i.display_name().to_lowercase().contains(&f))
                .collect()
        }
    }

    pub fn filtered_volumes(&self) -> Vec<&Volume> {
        if self.filter_text.is_empty() {
            self.volumes.iter().collect()
        } else {
            let f = self.filter_text.to_lowercase();
            self.volumes
                .iter()
                .filter(|v| v.name.to_lowercase().contains(&f))
                .collect()
        }
    }

    pub fn current_list_len(&self) -> usize {
        match self.active_section {
            ResourceSection::Containers => self.filtered_containers().len(),
            ResourceSection::Images => self.filtered_images().len(),
            ResourceSection::Volumes => self.filtered_volumes().len(),
        }
    }

    pub fn current_index(&self) -> usize {
        match self.active_section {
            ResourceSection::Containers => self.container_index,
            ResourceSection::Images => self.image_index,
            ResourceSection::Volumes => self.volume_index,
        }
    }

    pub fn set_current_index(&mut self, idx: usize) {
        match self.active_section {
            ResourceSection::Containers => self.container_index = idx,
            ResourceSection::Images => self.image_index = idx,
            ResourceSection::Volumes => self.volume_index = idx,
        }
    }

    pub fn move_up(&mut self) {
        let idx = self.current_index();
        if idx > 0 {
            self.set_current_index(idx - 1);
        } else {
            // At top of current section — move to previous section's last item
            match self.active_section {
                ResourceSection::Containers => {} // already at top
                ResourceSection::Images => {
                    let prev_len = self.filtered_containers().len();
                    if prev_len > 0 {
                        self.active_section = ResourceSection::Containers;
                        self.container_index = prev_len - 1;
                    }
                }
                ResourceSection::Volumes => {
                    let prev_len = self.filtered_images().len();
                    if prev_len > 0 {
                        self.active_section = ResourceSection::Images;
                        self.image_index = prev_len - 1;
                    }
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        let len = self.current_list_len();
        let idx = self.current_index();
        if len > 0 && idx < len - 1 {
            self.set_current_index(idx + 1);
        } else {
            // At bottom of current section — move to next section's first item
            match self.active_section {
                ResourceSection::Containers => {
                    if !self.filtered_images().is_empty() {
                        self.active_section = ResourceSection::Images;
                        self.image_index = 0;
                    }
                }
                ResourceSection::Images => {
                    if !self.filtered_volumes().is_empty() {
                        self.active_section = ResourceSection::Volumes;
                        self.volume_index = 0;
                    }
                }
                ResourceSection::Volumes => {} // already at bottom
            }
        }
    }

    pub fn next_section(&mut self) {
        self.active_section = match self.active_section {
            ResourceSection::Containers => ResourceSection::Images,
            ResourceSection::Images => ResourceSection::Volumes,
            ResourceSection::Volumes => ResourceSection::Containers,
        };
        self.detail_tab = DetailTab::Main;
    }

    pub fn prev_section(&mut self) {
        self.active_section = match self.active_section {
            ResourceSection::Containers => ResourceSection::Volumes,
            ResourceSection::Images => ResourceSection::Containers,
            ResourceSection::Volumes => ResourceSection::Images,
        };
        self.detail_tab = DetailTab::Main;
    }

    pub fn next_tab(&mut self) {
        self.detail_tab = match self.detail_tab {
            DetailTab::Main => DetailTab::Info,
            DetailTab::Info => DetailTab::Env,
            DetailTab::Env => DetailTab::Main,
        };
    }

    pub fn prev_tab(&mut self) {
        self.detail_tab = match self.detail_tab {
            DetailTab::Main => DetailTab::Env,
            DetailTab::Info => DetailTab::Main,
            DetailTab::Env => DetailTab::Info,
        };
    }

    pub fn clamp_indices(&mut self) {
        let clen = self.filtered_containers().len();
        if clen == 0 {
            self.container_index = 0;
        } else if self.container_index >= clen {
            self.container_index = clen - 1;
        }

        let ilen = self.filtered_images().len();
        if ilen == 0 {
            self.image_index = 0;
        } else if self.image_index >= ilen {
            self.image_index = ilen - 1;
        }

        let vlen = self.filtered_volumes().len();
        if vlen == 0 {
            self.volume_index = 0;
        } else if self.volume_index >= vlen {
            self.volume_index = vlen - 1;
        }
    }

    pub fn set_flash(&mut self, msg: String) {
        self.flash_message = Some(msg);
        self.flash_timer = 10; // ticks
    }

    pub fn tick_flash(&mut self) {
        if self.flash_timer > 0 {
            self.flash_timer -= 1;
            if self.flash_timer == 0 {
                self.flash_message = None;
            }
        }
    }

    pub fn build_action_menu(&mut self) {
        self.action_menu_items.clear();
        self.action_menu_index = 0;

        match self.active_section {
            ResourceSection::Containers => {
                if let Some(c) = self.selected_container() {
                    if c.is_running() {
                        self.action_menu_items.push(ActionMenuItem { label: "Stop".into(), hotkey: 'S' });
                        self.action_menu_items.push(ActionMenuItem { label: "Kill".into(), hotkey: 'K' });
                    } else {
                        self.action_menu_items.push(ActionMenuItem { label: "Start".into(), hotkey: 's' });
                    }
                    self.action_menu_items.push(ActionMenuItem { label: "Remove".into(), hotkey: 'x' });
                    self.action_menu_items.push(ActionMenuItem { label: "View Logs".into(), hotkey: 'l' });
                }
            }
            ResourceSection::Images => {
                self.action_menu_items.push(ActionMenuItem { label: "Pull Image".into(), hotkey: 'p' });
                if self.selected_image().is_some() {
                    self.action_menu_items.push(ActionMenuItem { label: "Remove".into(), hotkey: 'x' });
                }
            }
            ResourceSection::Volumes => {
                if self.selected_volume().is_some() {
                    self.action_menu_items.push(ActionMenuItem { label: "Remove".into(), hotkey: 'x' });
                }
            }
        }

        self.input_mode = InputMode::ActionMenu;
    }

    pub fn push_stats_sample(&mut self, container_id: &str, cpu: f64, mem: f64) {
        let history = self.stats_history.entry(container_id.to_string()).or_default();
        history.push_cpu(cpu);
        history.push_memory(mem);
    }

    pub fn get_stats_history(&self, container_id: &str) -> Option<&StatsHistory> {
        self.stats_history.get(container_id)
    }
}
