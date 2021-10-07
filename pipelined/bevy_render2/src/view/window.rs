use crate::{
    render_resource::TextureView,
    renderer::{RenderDevice, RenderInstance},
    texture::BevyDefault,
    RenderApp, RenderStage, RenderWorld,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_utils::{tracing::debug, HashMap};
use bevy_window::{RawWindowHandleWrapper, WindowId, Windows};
use std::ops::{Deref, DerefMut};
use wgpu::TextureFormat;

// Token to ensure a system runs on the main thread.
#[derive(Default)]
pub struct NonSendMarker;

pub struct WindowRenderPlugin;

impl Plugin for WindowRenderPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .init_resource::<ExtractedWindows>()
            .init_resource::<WindowSurfaces>()
            .init_resource::<NonSendMarker>()
            .add_system_to_stage(RenderStage::Extract, extract_windows)
            .add_system_to_stage(RenderStage::Prepare, prepare_windows);
    }
}

pub struct ExtractedWindow {
    pub id: WindowId,
    pub handle: RawWindowHandleWrapper,
    pub physical_width: u32,
    pub physical_height: u32,
    pub vsync: bool,
    pub swap_chain_frame: Option<TextureView>,
    pub size_changed: bool,
}

#[derive(Default)]
pub struct ExtractedWindows {
    pub windows: HashMap<WindowId, ExtractedWindow>,
}

impl Deref for ExtractedWindows {
    type Target = HashMap<WindowId, ExtractedWindow>;

    fn deref(&self) -> &Self::Target {
        &self.windows
    }
}

impl DerefMut for ExtractedWindows {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.windows
    }
}

fn extract_windows(mut render_world: ResMut<RenderWorld>, windows: Res<Windows>) {
    let mut extracted_windows = render_world.get_resource_mut::<ExtractedWindows>().unwrap();
    for window in windows.iter() {
        let (new_width, new_height) = (
            window.physical_width().max(1),
            window.physical_height().max(1),
        );

        let mut extracted_window =
            extracted_windows
                .entry(window.id())
                .or_insert(ExtractedWindow {
                    id: window.id(),
                    handle: window.raw_window_handle(),
                    physical_width: new_width,
                    physical_height: new_height,
                    vsync: window.vsync(),
                    swap_chain_frame: None,
                    size_changed: false,
                });

        // NOTE: Drop the swap chain frame here

        extracted_window.swap_chain_frame = None;
        extracted_window.size_changed = new_width != extracted_window.physical_width
            || new_height != extracted_window.physical_height;

        if extracted_window.size_changed {
            debug!(
                "Window size changed from {}x{} to {}x{}",
                extracted_window.physical_width,
                extracted_window.physical_height,
                new_width,
                new_height
            );
            extracted_window.physical_width = new_width;
            extracted_window.physical_height = new_height;
        }
    }
}

#[derive(Default)]
pub struct WindowSurfaces {
    surfaces: HashMap<WindowId, wgpu::Surface>,
    swap_chains: HashMap<WindowId, wgpu::SwapChain>,
}

pub fn prepare_windows(
    // By accessing a NonSend resource, we tell the scheduler to put this system on the main thread,
    // which is necessary for some OS s
    _marker: NonSend<NonSendMarker>,
    mut windows: ResMut<ExtractedWindows>,
    mut window_surfaces: ResMut<WindowSurfaces>,
    render_device: Res<RenderDevice>,
    render_instance: Res<RenderInstance>,
) {
    
    
    let window_surfaces = window_surfaces.deref_mut();
    for window in windows.windows.values_mut() {
        let surface = window_surfaces
            .surfaces
            .entry(window.id)
            .or_insert_with(|| unsafe {
                // NOTE: On some OSes this MUST be called from the main thread.
                render_instance.create_surface(&window.handle.get_handle())
            });

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            format: TextureFormat::bevy_default(),
            width: window.physical_width,
            height: window.physical_height,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            present_mode: if window.vsync {
                wgpu::PresentMode::Fifo
            } else {
                wgpu::PresentMode::Immediate
            },
        };

        if window.size_changed {
            window_surfaces.swap_chains.insert(
                window.id,
                render_device.create_swap_chain(surface, &swap_chain_descriptor),
            );
        }

        let swap_chain = window_surfaces
            .swap_chains
            .entry(window.id)
            .or_insert_with(|| render_device.create_swap_chain(surface, &swap_chain_descriptor));

        let frame = match swap_chain.get_current_frame() {
            Ok(swap_chain_frame) => swap_chain_frame,
            Err(wgpu::SwapChainError::Outdated) => {
                let new_swap_chain =
                    render_device.create_swap_chain(surface, &swap_chain_descriptor);
                let frame = new_swap_chain
                    .get_current_frame()
                    .expect("Error recreating swap chain");
                window_surfaces
                    .swap_chains
                    .insert(window.id, new_swap_chain);
                frame
            }
            err => err.expect("Failed to acquire next swap chain texture!"),
        };

        window.swap_chain_frame = Some(TextureView::from(frame));
    }
}
