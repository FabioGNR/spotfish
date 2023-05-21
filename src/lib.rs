use instant::Instant;

use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader, console, WebGlBuffer};
use serde::{Serialize, Deserialize};
struct StaticVariables {
    canvas_size: [f32; 2]
}

const MAX_SECTIONS: usize = 64;
const MAX_SEGMENTS: usize = 160;

#[wasm_bindgen]
struct DynamicVariables {
    time: f32,
    song_position: f32,
    song_sections: Vec<SongSection>
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct SongSection {
    pub start: f32,
    pub duration: f32,
    pub loudness: f32,
    pub tempo: f32
}

#[derive(Serialize, Deserialize)]
pub struct SongSegment {
    pub start: f32,
    pub duration: f32,
    pub pitches: [f32; 12],
    pub timbre: [f32; 12]
}

#[wasm_bindgen]
pub struct Instance {
    canvas: web_sys::HtmlCanvasElement,
    context: WebGl2RenderingContext,
    program: WebGlProgram,
    song_sections_buffer: WebGlBuffer,
    song_segments_buffer: WebGlBuffer,
    vert_count: i32,
    dynamic_vars: DynamicVariables,
    start_time: Instant,
    song_state_time: Instant,
    song_position_offset: f32,
}

#[wasm_bindgen]
impl Instance {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: web_sys::HtmlCanvasElement, vert_shader: String, frag_shader: String) -> Result<Instance, JsValue> {
        let context = canvas
            .get_context("webgl2")?
            .unwrap()
            .dyn_into::<WebGl2RenderingContext>()?;

        let vert_shader = compile_shader(
            &context,
            WebGl2RenderingContext::VERTEX_SHADER,
            &vert_shader,
        )?;
    
        let frag_shader = compile_shader(
            &context,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            &frag_shader,
        )?;
        let program = link_program(&context, &vert_shader, &frag_shader)?;
        context.use_program(Some(&program));

        let vertices = vec![
            // First triangle:
             1.0,  1.0,
            -1.0,  1.0,
            -1.0, -1.0,
            // Second triangle:
            -1.0, -1.0,
             1.0, -1.0,
             1.0,  1.0
        ];

        let vert_count = (vertices.len() / 2) as i32;

        init_vertices(&context, &program, vertices)?;

        // init song sections buffer
        let sections_index = context.get_uniform_block_index(&program, "SongSections");
        let song_sections_buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        context.bind_buffer_base(WebGl2RenderingContext::UNIFORM_BUFFER, sections_index, Some(&song_sections_buffer));
        context.buffer_data_with_i32(WebGl2RenderingContext::UNIFORM_BUFFER, (::core::mem::size_of::<SongSection>() * MAX_SECTIONS) as i32, WebGl2RenderingContext::DYNAMIC_DRAW);
        // init song segments buffer
        // let segments_index = context.get_uniform_block_index(&program, "SongSegments");
        let song_segments_buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        // context.bind_buffer_base(WebGl2RenderingContext::UNIFORM_BUFFER, segments_index, Some(&song_segments_buffer));
        // context.buffer_data_with_i32(WebGl2RenderingContext::UNIFORM_BUFFER, (::core::mem::size_of::<SongSegment>() * MAX_SEGMENTS) as i32, WebGl2RenderingContext::DYNAMIC_DRAW);
        // console::log_2(&"bla".into(), &JsValue::from_f64((::core::mem::size_of::<SongSegment>() * MAX_SEGMENTS) as f64));

        let instance = Instance {
            canvas,
            context,
            program,
            song_sections_buffer,
            song_segments_buffer,
            vert_count,
            dynamic_vars: DynamicVariables { time: 0.0, song_position: 0.0, song_sections: vec![] },
            start_time: Instant::now(),
            song_state_time: Instant::now(),
            song_position_offset: 0.0
        };

        instance.update_static();

        Ok(instance)
    }

    pub fn update_static(&self) {
        let static_vars = StaticVariables {
            canvas_size: [self.canvas.width() as f32, self.canvas.height() as f32]
        };
    
        upload_static_uniforms(&self.context, &self.program, static_vars);
    }

    pub fn set_song(&mut self, sections: JsValue, segments: JsValue, position: f32) -> Result<(), JsValue> {
        self.song_state_time = Instant::now();

        let mut sections: Vec<SongSection> = serde_wasm_bindgen::from_value(sections).expect("sections not able to deserialize into SongSection");
        let mut segments: Vec<SongSegment> = serde_wasm_bindgen::from_value(segments).expect("segments not able to deserialize into SongSegment");

        sections.truncate(MAX_SECTIONS);
        segments.truncate(MAX_SEGMENTS);

        let num_sections_uniform = self.context.get_uniform_location(&self.program, "numSections");
        self.context.uniform1i(num_sections_uniform.as_ref(), sections.len() as i32);
    
        let sections_index = self.context.get_uniform_block_index(&self.program, "SongSections");
        self.context.bind_buffer_base(WebGl2RenderingContext::UNIFORM_BUFFER, sections_index, Some(&self.song_sections_buffer));
        unsafe {
            let bytes = ::core::slice::from_raw_parts(
                sections.as_ptr() as *const u8,
                ::core::mem::size_of::<SongSection>() * sections.len(),
            );
    
            self.context.buffer_sub_data_with_i32_and_u8_array(
                WebGl2RenderingContext::UNIFORM_BUFFER,
                0,
                bytes,
            );
        }

        // let num_segments_uniform = self.context.get_uniform_location(&self.program, "numSegments");
        // self.context.uniform1i(num_segments_uniform.as_ref(), segments.len() as i32);
    
        // let segments_index = self.context.get_uniform_block_index(&self.program, "SongSegments");
        // self.context.bind_buffer_base(WebGl2RenderingContext::UNIFORM_BUFFER, segments_index, Some(&self.song_segments_buffer));
        // unsafe {
        //     let bytes = ::core::slice::from_raw_parts(
        //         segments.as_ptr() as *const u8,
        //         ::core::mem::size_of::<SongSegment>() * segments.len(),
        //     );
    
        //     self.context.buffer_sub_data_with_i32_and_u8_array(
        //         WebGl2RenderingContext::UNIFORM_BUFFER,
        //         0,
        //         bytes,
        //     );
        // }

        self.dynamic_vars.song_sections = sections;
        self.dynamic_vars.song_position = position;
        self.song_position_offset = position;

        upload_dynamic_uniforms(&self.context, &self.program, &self.dynamic_vars)
    }

    pub fn draw(&mut self) -> Result<(), JsValue> {
        let current_time = Instant::now();
        let elapsed_time = current_time - self.start_time;
        self.dynamic_vars.time = elapsed_time.as_secs_f32();

        let elapsed_song_time = current_time - self.song_state_time;
        self.dynamic_vars.song_position = self.song_position_offset + elapsed_song_time.as_secs_f32();
        upload_dynamic_uniforms(&self.context, &self.program, &self.dynamic_vars)?;

        draw(&self.context, self.vert_count);
        Ok(())
    }
}

fn upload_static_uniforms(context: &WebGl2RenderingContext, program: &WebGlProgram, variables: StaticVariables) {
    let canvas_size_uniform = context.get_uniform_location(&program, "canvasSize");
    context.uniform2fv_with_f32_array(canvas_size_uniform.as_ref(), variables.canvas_size.as_slice());  
}

fn get_current_section_index(variables: &DynamicVariables) -> u32 {
    let current_section = variables.song_sections.iter()
        .position(|s| s.start < variables.song_position && s.start + s.duration > variables.song_position);

    return current_section.unwrap_or(0) as u32;
}

fn upload_dynamic_uniforms(context: &WebGl2RenderingContext, program: &WebGlProgram, variables: &DynamicVariables) -> Result<(), JsValue> {
    let time_uniform = context.get_uniform_location(&program, "time");
    context.uniform1f(time_uniform.as_ref(), variables.time);
    
    let song_time_uniform = context.get_uniform_location(&program, "songTime");
    context.uniform1f(song_time_uniform.as_ref(), variables.song_position);

    let current_section_uniform = context.get_uniform_location(&program, "currentSongSection");
    context.uniform1ui(current_section_uniform.as_ref(), get_current_section_index(&variables));

    Ok(())
}

fn draw(context: &WebGl2RenderingContext, vert_count: i32) {
    context.clear_color(0.0, 0.0, 0.0, 1.0);
    context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

    context.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, vert_count);
}

pub fn init_vertices(context: &WebGl2RenderingContext, program: &WebGlProgram, vertices: Vec<f32>) -> Result<(), JsValue> {
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

    Ok(())
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
