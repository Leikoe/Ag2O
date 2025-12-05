use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::NSView;
use objc2_foundation::{NSString, NSURL};
use objc2_metal::{
    MTL4CommandAllocator, MTL4CommandBuffer, MTL4CommandEncoder, MTL4CommandQueue, MTL4Compiler,
    MTL4CompilerDescriptor, MTL4LibraryDescriptor, MTL4LibraryFunctionDescriptor,
    MTL4RenderCommandEncoder, MTL4RenderPassDescriptor, MTL4RenderPipelineDescriptor,
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue,
    MTLCreateSystemDefaultDevice, MTLDevice, MTLDrawable, MTLLibrary, MTLLoadAction,
    MTLPrimitiveType, MTLRenderPassDescriptor, MTLRenderPipelineState, MTLTexture, MTLViewport,
};
use objc2_metal_kit::{MTKTextureLoader, MTKView};

use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes};

const SHADER_SOURCE: &str = include_str!("./shader.metal");

struct App {
    window: Option<Window>,
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    metal_layer: Option<Retained<CAMetalLayer>>,
    current_image: Option<Retained<ProtocolObject<dyn MTLTexture>>>,
    render_pipeline_state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("resumed!");
        if self.window.is_none() {
            let window = event_loop
                .create_window(WindowAttributes::default())
                .unwrap();

            // setup a view on which metal can draw
            let handle = window.window_handle().unwrap();
            let parent_view = match handle.as_raw() {
                raw_window_handle::RawWindowHandle::AppKit(app_kit_window_handle) => {
                    let ptr = app_kit_window_handle.ns_view.as_ptr();
                    let view = unsafe { Retained::retain(ptr.cast::<NSView>()).unwrap() };
                    view
                }
                _ => {
                    panic!("Only MacOS is supported!");
                }
            };

            let layer = CAMetalLayer::new();
            layer.setDevice(Some(&self.device));
            layer.setOpaque(true); // "An opaque layer can provide direct-to-display contents under the right conditions" https://developer.apple.com/documentation/metal/managing-your-game-window-for-metal-in-macos?language=objc#Display-your-Metal-content-in-your-new-view
            layer.setPixelFormat(objc2_metal::MTLPixelFormat::BGRA8Unorm);
            layer.setFrame(parent_view.frame()); // Set the initial size to the same as winit windw

            // attach the layer to the winit window
            parent_view.setWantsLayer(true); // whether the view uses a layer as its backing store
            parent_view.setLayer(Some(&layer));

            // setup command queue and submit one render to it
            let command_queue = self.device.newMTL4CommandQueue().unwrap();

            let command_buffer_allocator = self.device.newCommandAllocator().unwrap();
            command_buffer_allocator.reset();
            let command_buffer = self.device.newCommandBuffer().unwrap();
            command_buffer.beginCommandBufferWithAllocator(&command_buffer_allocator);

            let drawable = layer.nextDrawable().unwrap();
            let encoder_desc = {
                let desc = MTL4RenderPassDescriptor::new();
                let color_att = unsafe { desc.colorAttachments().objectAtIndexedSubscript(0) };
                color_att.setTexture(Some(&drawable.texture()));
                color_att.setLoadAction(MTLLoadAction::Clear);
                color_att.setClearColor(MTLClearColor {
                    red: 1.0,
                    green: 1.0,
                    blue: 0.1,
                    alpha: 1.0,
                });
                desc
            };
            let encoder = command_buffer
                .renderCommandEncoderWithDescriptor(&encoder_desc)
                .unwrap();
            encoder.setRenderPipelineState(self.render_pipeline_state.as_ref());
            encoder.endEncoding();
            command_buffer.endCommandBuffer();

            // Finally, launch the work!
            // 1 - Wait for the drawable to be ready
            command_queue.waitForDrawable(drawable.as_ref());
            // 2 - submit the buffer
            let mut cmd_buffers = [command_buffer];
            unsafe {
                command_queue.commit_count(
                    NonNull::new(cmd_buffers.as_mut_ptr().cast()).unwrap(),
                    cmd_buffers.len(),
                );
            }
            // 3 - Notify the drawable that the GPU is done running the render pass
            command_queue.signalDrawable(drawable.as_ref());

            // Instruct the drawable to show itself on the device's display when the render pass completes.
            drawable.present();

            self.window = Some(window);
            self.metal_layer = Some(layer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        println!("window_event {:?}", event);
        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        };
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let device = MTLCreateSystemDefaultDevice().expect("failed to get default system device");
    println!("{:?}", device);

    // create a MTL4Compiler
    let compiler = device
        .newCompilerWithDescriptor_error(&MTL4CompilerDescriptor::new())
        .unwrap();

    // compile our shader into a library
    let library_desc = MTL4LibraryDescriptor::new();
    let src_string = NSString::from_str(SHADER_SOURCE);
    library_desc.setSource(Some(&src_string));
    let library = compiler
        .newLibraryWithDescriptor_error(&library_desc)
        .unwrap();

    // describe and compile our render pipeline state
    let fragment_function_desc = {
        let d = MTL4LibraryFunctionDescriptor::new();
        d.setLibrary(Some(&library));
        d.setName(Some(&NSString::from_str("fragment_main")));
        d
    };
    let vertex_function_desc = {
        let d = MTL4LibraryFunctionDescriptor::new();
        d.setLibrary(Some(&library));
        d.setName(Some(&NSString::from_str("vertex_main")));
        d
    };
    let render_pipeline_desc = MTL4RenderPipelineDescriptor::new();
    render_pipeline_desc.setFragmentFunctionDescriptor(Some(&fragment_function_desc));
    render_pipeline_desc.setVertexFunctionDescriptor(Some(&vertex_function_desc));
    let render_pipeline_state = compiler
        .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(&render_pipeline_desc, None)
        .unwrap();

    // let tl = unsafe { MTKTextureLoader::alloc() };
    // let tl = MTKTextureLoader::initWithDevice(tl, &device);

    // let path = NSString::from_str("/Users/leo/git/Ag2O/for_valued_client.png");
    // let path = NSString::from_str("/Users/leo/git/Ag2O/lunar_lake.jpg");
    // let url = NSURL::fileURLWithPath(&path);
    // let texture = unsafe {
    //     tl.newTextureWithContentsOfURL_options_error(&url, None)
    //         .unwrap()
    // };
    // dbg!(&texture);

    // let command_queue = device.newMTL4CommandQueue().unwrap();
    // let command_buffer = device.newCommandBuffer().unwrap();

    // let argument_table_desc = MTL4ArgumentTableDescriptor::new();
    // argument_table_desc.setMaxTextureBindCount(1);

    // let argument_table = device
    //     .newArgumentTableWithDescriptor_error(&argument_table_desc)
    //     .unwrap();
    // unsafe { argument_table.setTexture_atIndex(texture.gpuResourceID(), 0) };

    // let command_allocator = device.newCommandAllocator().unwrap();
    // command_allocator.reset();
    // command_buffer.beginCommandBufferWithAllocator(&command_allocator);

    // let render_pass_descriptor = MTL4RenderPassDescriptor::new();
    // unsafe {
    //     render_pass_descriptor
    //         .colorAttachments()
    //         .objectAtIndexedSubscript(0)
    //         .setLoadAction(MTLLoadAction::Clear);
    //     render_pass_descriptor
    //         .colorAttachments()
    //         .objectAtIndexedSubscript(0)
    //         .setClearColor(MTLClearColor {
    //             red: 0.0,
    //             green: 0.0,
    //             blue: 1.0,
    //             alpha: 1.0,
    //         });
    // }

    // let render_pass_encoder = command_buffer
    //     .renderCommandEncoderWithDescriptor(&render_pass_descriptor)
    //     .unwrap();

    // render_pass_encoder.endEncoding();
    // command_buffer.endCommandBuffer();

    // println!("{:?}", texture);

    let mut app = App {
        window: None,
        device,
        metal_layer: None,
        current_image: None,
        render_pipeline_state,
    };

    let _ = event_loop.run_app(&mut app);
}
