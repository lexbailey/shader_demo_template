extern crate gl;
extern crate glutin;

use gl::types::*;
use std::mem;
use std::ptr;
use std::os::raw::c_void;
use std::time::{Instant,Duration};
use std::cell::RefCell;
use std::ffi::CString;
use glutin::ContextWrapper;
use glutin::PossiblyCurrent;

use gl_abstractions as gla;
use gla::{
    shader_struct,
    impl_shader,
    UniformFloat,
    UniformMat4,
    UniformVec2,
    // Add in other uniform types as required
    //UniformVec3,
    //UniformVec4,
    //UniformSampler2D,
};

#[cfg(debug_assertions)]
extern "system" fn gl_error_handler(_source: u32, _type_: u32, _id: u32, _severity: u32, _length: i32, message: *const i8, _user: *mut std::ffi::c_void){
    let message = unsafe{std::ffi::CStr::from_ptr(message)};
    println!("[OpenGL] {}", message.to_string_lossy());
}

// This is an example shader program
// if you want to load the source from a separate file, you can use `include_str!("shader.frag")` in place of a string literal below
// Currently this only supports one specific pipeline config: a vertex shader connected to a fragment shader
shader_struct!{
    ExampleShader,
    // The first argument is a vertex shader
    r#"
        #version 330 core
        layout (location = 0) in vec2 aPos;
        uniform mat4 u_global_transform;
        uniform float u_time;
        uniform vec2 u_offset;
        uniform vec2 u_winsize;
        out float time;
        out vec2 coord_offset;
        out vec2 winsize;
        void main()
        {
            // transform for only shading the largest rectangle of the right ratio in the middle of the screen
            gl_Position = vec4(aPos,0.0,1.0)  * u_global_transform;
            // pass time to frag shader
            time = u_time;
            coord_offset = u_offset;
            winsize = u_winsize;
        }
        "#,
    // The second argument is the fragment shader
    r#"
        #version 330 core
        in float time; // time in seconds since the start of the program
        in vec2 coord_offset; 
        in vec2 winsize; 
        out vec4 FragColor;
        void main()
        {
            // screen coordinates (scaled to remain consistent when window resizes, 1:1 with pixels at default window size)
            vec2 pxcoord = (gl_FragCoord.xy - coord_offset);
            vec2 coord = pxcoord/winsize;

            // shader logic
            float r = 0.5*(sin((coord.x*10.0) + time)+1.0);
            float g = 0.5*(sin((coord.y*20.0) + time+1.5)+1.0);
            float b = 0.5*(sin(time+3.0)+1.0);

            // output colour
            FragColor = vec4(r,g,b, 1.0);
        }
        "#,
    // Finally is a list of uniforms (for passing data in to the shader)
    // this example provides a global transform that maintains the initial aspect ratio when the window size changes
    {
        u_global_transform: UniformMat4,
        u_time: UniformFloat,
        u_offset: UniformVec2,
        u_winsize: UniformVec2,
        /* u_custom_variable: UniformFloat */
    }
}

struct RenderData{
    shader: ExampleShader,
    window: ContextWrapper<PossiblyCurrent, glutin::window::Window>,
    events_loop: RefCell<Option<glutin::event_loop::EventLoop<()>>>,
    ratio: f32,
}

fn init_render_data(start_fullscreen: bool, start_w: u32,start_h: u32) -> RenderData{
    let events_loop = glutin::event_loop::EventLoop::with_user_event();
    let window = glutin::window::WindowBuilder::new()
        .with_title("Example Shader");
    let window = if start_fullscreen {
        window.with_fullscreen(Some(glutin::window::Fullscreen::Borderless(None)))
    }
    else{
        let w = if start_w != 0 {start_w} else {1920};
        let h = if start_h != 0 {start_h} else {1080};
        window.with_inner_size(glutin::dpi::PhysicalSize::new(w,h))
    };
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let gl_window = unsafe {
        let win = context.build_windowed(window, &events_loop).unwrap().make_current().unwrap();
        gl::load_with(|s| win.get_proc_address(s) as *const _);
        win
    };

    let shader = ExampleShader::new();

    let gfx_objs = unsafe{ 
        #[cfg(debug_assertions)]
        {
            if gl::DebugMessageCallback::is_loaded(){
                gl::Enable(gl::DEBUG_OUTPUT);
                gl::DebugMessageCallback(Some(gl_error_handler), std::ptr::null());
            }
        }
        let ratio = (start_w as GLfloat)/(start_h as GLfloat);
        // a rectangle to fill the view
        let vertices: [GLfloat;8] = [
            -ratio, -1.0,
            -ratio, 1.0,
            ratio, 1.0,
            ratio, -1.0,
        ];

        let mut vbo = 0;
        let mut verts = 0;
        gl::GenVertexArrays(1, &mut verts);
        gl::GenBuffers(1, &mut vbo);
        gl::BindVertexArray(verts);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            &vertices[0] as *const GLfloat as *const c_void,
            gl::STATIC_DRAW,
        );
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 2 * mem::size_of::<GLfloat>() as GLsizei, ptr::null());

        RenderData{
            shader,
            window: gl_window,
            events_loop: RefCell::new(Some(events_loop)),
            ratio,
        }
    };
    gfx_objs
}

type Tf = affine::Transform<GLfloat>;

fn main() {
    let mut gfx = init_render_data(/*fullscreen ->*/ false, 1920, 1080);

    fn draw(gfx: &mut RenderData, t: std::time::Duration){
        let sz = gfx.window.window().inner_size();
        let ww = sz.width as f32;
        let wh = sz.height as f32;

        let (global_transform, offset, newsize): (_, (GLfloat, GLfloat), _) = if ww < (wh * gfx.ratio){
            let nh =  ww / gfx.ratio;
            (
                Tf::scale(1.0/gfx.ratio, nh/wh,1.0),
                (0.0,(wh - nh)/2.0),
                (ww,nh),
            )
        }
        else{
            let nw = wh * gfx.ratio;
            (
                Tf::scale(wh / ww,1.0,1.0),
                ((ww - nw)/2.0,0.0),
                (nw,wh),
            )
        };

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::ClearColor(0.6,0.6,0.6,1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gfx.shader.use_();
            gfx.shader.u_global_transform.set(&global_transform.data);
            gfx.shader.u_time.set(t.as_secs_f32());
            gfx.shader.u_offset.set(offset.0,offset.1);
            gfx.shader.u_winsize.set(newsize.0,newsize.1);
            gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
        }
        gfx.window.swap_buffers().unwrap();
    }

    let target_fps = 30.0;
    let frame_dur_ms: f32 = 1000.0/target_fps;

    let mut last_frame_start = Instant::now();

    let frame_duration = Duration::from_millis(frame_dur_ms.floor() as u64);

    use glutin::event::{Event, WindowEvent};
    use glutin::event_loop::ControlFlow;
    let events_loop = gfx.events_loop.take().unwrap();

    let start_time = Instant::now();
    events_loop.run(move |event, _win_target, cf|
        match event {
            Event::WindowEvent{ event: ev,..} => {
                match ev {
                    WindowEvent::CloseRequested => {*cf = ControlFlow::Exit;}
                    ,WindowEvent::Resized(newsize) => {
                        gfx.window.resize(newsize);
                        unsafe{ gl::Viewport(0,0,newsize.width as i32, newsize.height as i32); }
                    }
                    ,WindowEvent::KeyboardInput{input: glutin::event::KeyboardInput{virtual_keycode:Some(glutin::event::VirtualKeyCode::Escape), ..}, ..} => {
                        std::process::exit(0);
                    }
                    ,_=>{}
                }
            },
            Event::RedrawRequested(_win) => {
                let t = Instant::now() - start_time;
                draw(&mut gfx, t);
            },
            Event::RedrawEventsCleared => {
                let start = Instant::now();
                let t = Instant::now() - start_time;
                draw(&mut gfx, t);
                *cf = ControlFlow::WaitUntil(last_frame_start+frame_duration); last_frame_start = start;
            },
            _ => {},
        }
    );

}

