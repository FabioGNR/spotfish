use std::{cell::RefCell, rc::Rc};
use instant::Instant;

use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader, console};

struct StaticVariables {
    canvas_size: [f32; 2]
}

struct DynamicVariables {
    time: f32
}


fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window().expect("missing window")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    canvas.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
    canvas.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);
    
    let context = canvas
        .get_context("webgl2")?
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()?;
    
    let vert_shader = compile_shader(
        &context,
        WebGl2RenderingContext::VERTEX_SHADER,
        r##"#version 300 es
 
        in vec4 position;

        void main() {
        
            gl_Position = position;
        }
        "##,
    )?;

    let frag_shader = compile_shader(
        &context,
        WebGl2RenderingContext::FRAGMENT_SHADER,
        r##"#version 300 es
    
        precision highp float;
        uniform vec2 canvasSize;
        uniform float time;

        out vec4 outColor;
        
        void main() {
            vec2 pos = gl_FragCoord.xy / canvasSize;
            outColor = vec4(0.5+sin(time+pos.x), 0.5+cos(time+pos.y), 1.0-(0.5+sin(time+pos.x+pos.y)), 1);
        }
        "##,
    )?;
    let program = link_program(&context, &vert_shader, &frag_shader)?;
    context.use_program(Some(&program));

    let static_vars = StaticVariables {
        canvas_size: [canvas.width() as f32, canvas.height() as f32]
    };

    upload_static_uniforms(&context, &program, static_vars);

    let vertices: [f32; 12] = [
        // First triangle:
         1.0,  1.0,
        -1.0,  1.0,
        -1.0, -1.0,
        // Second triangle:
        -1.0, -1.0,
         1.0, -1.0,
         1.0,  1.0
    ];

    let position_attribute_location = context.get_attrib_location(&program, "position");
    let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
    context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

    // Note that `Float32Array::view` is somewhat dangerous (hence the
    // `unsafe`!). This is creating a raw view into our module's
    // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
    // (aka do a memory allocation in Rust) it'll cause the buffer to change,
    // causing the `Float32Array` to be invalid.
    //
    // As a result, after `Float32Array::view` we have to be very careful not to
    // do any memory allocations before it's dropped.
    unsafe {
        let positions_array_buf_view = js_sys::Float32Array::view(&vertices);

        context.buffer_data_with_array_buffer_view(
            WebGl2RenderingContext::ARRAY_BUFFER,
            &positions_array_buf_view,
            WebGl2RenderingContext::STATIC_DRAW,
        );
    }

    let vao = context
        .create_vertex_array()
        .ok_or("Could not create vertex array object")?;
    context.bind_vertex_array(Some(&vao));

    context.vertex_attrib_pointer_with_i32(
        position_attribute_location as u32,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        0,
        0,
    );
    context.enable_vertex_attrib_array(position_attribute_location as u32);

    context.bind_vertex_array(Some(&vao));

    let vert_count = (vertices.len() / 2) as i32;
    draw(&context, vert_count);

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let mut dynamic_vars = DynamicVariables {
        time: 0.0
    };

    let start_time = Instant::now();

    *g.borrow_mut() = Some(Closure::new(move || {

        let current_time = Instant::now();
        let elapsed_time = current_time - start_time;
        dynamic_vars.time = elapsed_time.as_secs_f32();
        upload_dynamic_uniforms(&context, &program, &dynamic_vars);

        draw(&context, vert_count);
        // Schedule ourself for another requestAnimationFrame callback.
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());

    Ok(())
}

fn upload_static_uniforms(context: &WebGl2RenderingContext, program: &WebGlProgram, variables: StaticVariables) {
    let canvas_size_uniform = context.get_uniform_location(&program, "canvasSize");
    context.uniform2fv_with_f32_array(canvas_size_uniform.as_ref(), variables.canvas_size.as_slice());  
}

fn upload_dynamic_uniforms(context: &WebGl2RenderingContext, program: &WebGlProgram, variables: &DynamicVariables) {
    let time_uniform = context.get_uniform_location(&program, "time");
    context.uniform1f(time_uniform.as_ref(), variables.time);
}

fn draw(context: &WebGl2RenderingContext, vert_count: i32) {
    context.clear_color(0.0, 0.0, 0.0, 1.0);
    context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

    context.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, vert_count);
}

pub fn compile_shader(
    context: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
