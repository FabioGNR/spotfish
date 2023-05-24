use instant::Instant;

use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader, console, WebGlBuffer};
use serde::{Serialize, Deserialize};
use std140;

struct StaticVariables {
    canvas_size: [f32; 2]
}

const MAX_SECTIONS: usize = 64;
const MAX_SEGMENTS: usize = 100;

#[wasm_bindgen]
struct DynamicVariables {
    time: f32,
    song_position: f32,
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct SongSection {
    pub start: f32,
    pub duration: f32,
    pub loudness: f32,
    pub tempo: f32
}

#[std140::repr_std140]
struct SongSectionGpu {
    start: std140::float,
    duration: std140::float,
    loudness: std140::float,
    tempo: std140::float
}


impl From<&SongSection> for SongSectionGpu {
    fn from(value: &SongSection) -> Self {
        Self {
            start: std140::float(value.start),
            duration: std140::float(value.duration),
            loudness: std140::float(value.loudness),
            tempo: std140::float(value.tempo),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SongSegment {
    pub start: f32,
    pub duration: f32,
    pub loudness_max_time: f32,
    pub pitches: [f32; 12],
    pub timbre: [f32; 12],
}

#[std140::repr_std140]
struct SongSegmentGpu {
    pub start: std140::float,
    pub duration: std140::float,
    pub loudness_max_time: std140::float,
    pub pitches: std140::array<std140::vec4, 3>,
    pub timbre: std140::array<std140::vec4, 3>,
}

impl SongSegmentGpu {
    fn array_to_vec4s(floats: &[f32; 12]) -> std140::array<std140::vec4, 3> {
        std140::array![
            std140::vec4(floats[0], floats[1], floats[2], floats[3]),
            std140::vec4(floats[4], floats[5], floats[6], floats[7]),
            std140::vec4(floats[8], floats[9], floats[10], floats[11]),
        ]
    }
}

impl From<&SongSegment> for SongSegmentGpu {
    fn from(value: &SongSegment) -> Self {
        Self {
            start: std140::float(value.start),
            duration: std140::float(value.duration),
            loudness_max_time: std140::float(value.loudness_max_time),
            pitches: Self::array_to_vec4s(&value.pitches),
            timbre: Self::array_to_vec4s(&value.timbre),
        }
    }
}

struct SongData {
    sections: Vec<SongSection>,
    segments: Vec<SongSegment>
}

impl SongData {
    fn new() -> Self {
        Self {
            sections: vec![],
            segments: vec![],
        }
    }
}

struct GpuData {
    sections: Vec<SongSectionGpu>,
    segments: Vec<SongSegmentGpu>
}

impl GpuData {
    fn new() -> Self {
        Self {
            sections: vec![],
            segments: vec![],
        }
    }
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
    song_data: Box<SongData>,
    gpu_data: Box<GpuData>
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
        context.uniform_block_binding(&program, sections_index, sections_index);
        context.bind_buffer_base(WebGl2RenderingContext::UNIFORM_BUFFER, sections_index, Some(&song_sections_buffer));
        context.buffer_data_with_i32(WebGl2RenderingContext::UNIFORM_BUFFER, (::core::mem::size_of::<SongSectionGpu>() * MAX_SECTIONS) as i32, WebGl2RenderingContext::DYNAMIC_DRAW);

        // init song segments buffer
        let segments_index = context.get_uniform_block_index(&program, "SongSegments");
        let song_segments_buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        context.uniform_block_binding(&program, segments_index, segments_index);
        context.bind_buffer_base(WebGl2RenderingContext::UNIFORM_BUFFER, segments_index, Some(&song_segments_buffer));
        context.buffer_data_with_i32(WebGl2RenderingContext::UNIFORM_BUFFER, (::core::mem::size_of::<SongSegmentGpu>() * MAX_SEGMENTS) as i32, WebGl2RenderingContext::DYNAMIC_DRAW);

        let instance = Instance {
            canvas,
            context,
            program,
            song_sections_buffer,
            song_segments_buffer,
            vert_count,
            dynamic_vars: DynamicVariables { time: 0.0, song_position: 0.0 },
            start_time: Instant::now(),
            song_state_time: Instant::now(),
            song_position_offset: 0.0,
            song_data: Box::new(SongData::new()),
            gpu_data: Box::new(GpuData::new())
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
        sections.truncate(MAX_SECTIONS);
        let segments: Vec<SongSegment> = serde_wasm_bindgen::from_value(segments).expect("segments not able to deserialize into SongSegment");

        let gpu_sections: Vec<SongSectionGpu> = sections.iter().map(|s| s.into()).collect();
        let gpu_segments: Vec<SongSegmentGpu> = segments.iter().map(|s| s.into()).collect();

        let num_sections_uniform = self.context.get_uniform_location(&self.program, "numSections");
        self.context.uniform1ui(num_sections_uniform.as_ref(), sections.len() as u32);
    
        let sections_index = self.context.get_uniform_block_index(&self.program, "SongSections");
        self.context.bind_buffer_base(WebGl2RenderingContext::UNIFORM_BUFFER, sections_index, Some(&self.song_sections_buffer));
        unsafe {
            let bytes = ::core::slice::from_raw_parts(
                gpu_sections.as_ptr() as *const u8,
                ::core::mem::size_of::<SongSectionGpu>() * gpu_sections.len(),
            );
    
            self.context.buffer_sub_data_with_i32_and_u8_array(
                WebGl2RenderingContext::UNIFORM_BUFFER,
                0,
                bytes,
            );
        }

        self.song_data.sections = sections;
        self.song_data.segments = segments;
        self.dynamic_vars.song_position = position;
        self.song_position_offset = position;

        self.gpu_data.sections = gpu_sections;
        self.gpu_data.segments = gpu_segments;

        upload_dynamic_uniforms(&self.context, &self.program, &self.dynamic_vars)
    }

    pub fn print_song_time(&self) {
        console::log_2(&"Song time:".into(), &self.dynamic_vars.song_position.into());
    }

    fn get_current_section_index(&self) -> u32 {
        let current_section = self.song_data.sections.iter()
            .position(|s| s.start < self.dynamic_vars.song_position && s.start + s.duration > self.dynamic_vars.song_position);
    
        return current_section.unwrap_or(0) as u32;
    }

    fn get_current_segment_index(&self) -> usize {
        let current_segment = self.song_data.segments.iter()
            .position(|s| s.start < self.dynamic_vars.song_position && s.start + s.duration > self.dynamic_vars.song_position);

        return current_segment.unwrap_or(0);
    }
   
    pub fn draw(&mut self) -> Result<(), JsValue> {
        let current_time = Instant::now();
        let elapsed_time = current_time - self.start_time;
        self.dynamic_vars.time = elapsed_time.as_secs_f32();

        let elapsed_song_time = current_time - self.song_state_time;
        self.dynamic_vars.song_position = self.song_position_offset + elapsed_song_time.as_secs_f32();
        upload_dynamic_uniforms(&self.context, &self.program, &self.dynamic_vars)?;

        let current_section_uniform = self.context.get_uniform_location(&self.program, "currentSongSection");
        self.context.uniform1ui(current_section_uniform.as_ref(), self.get_current_section_index());

        let current_segment = self.get_current_segment_index();
        let start_segment = std::cmp::max(current_segment - 3, 0);
        let end_segment = std::cmp::min(start_segment + 10, self.song_data.segments.len());

        let num_segments_uniform = self.context.get_uniform_location(&self.program, "numSegments");
        self.context.uniform1ui(num_segments_uniform.as_ref(), (end_segment - start_segment) as u32);
    
        let current_segment_uniform = self.context.get_uniform_location(&self.program, "currentSongSegment");
        self.context.uniform1ui(current_segment_uniform.as_ref(), (current_segment - start_segment) as u32);
    
        let segments_index = self.context.get_uniform_block_index(&self.program, "SongSegments");
        self.context.bind_buffer_base(WebGl2RenderingContext::UNIFORM_BUFFER, segments_index, Some(&self.song_segments_buffer));
        unsafe {
            let bytes = ::core::slice::from_raw_parts(
                (self.gpu_data.segments.as_ptr().offset(start_segment as isize)) as *const u8,
                ::core::mem::size_of::<SongSegmentGpu>() * (end_segment - start_segment),
            );
    
            self.context.buffer_sub_data_with_i32_and_u8_array(
                WebGl2RenderingContext::UNIFORM_BUFFER,
                0,
                bytes
            );
        }

        draw(&self.context, self.vert_count);
        Ok(())
    }
}

fn upload_static_uniforms(context: &WebGl2RenderingContext, program: &WebGlProgram, variables: StaticVariables) {
    let canvas_size_uniform = context.get_uniform_location(&program, "canvasSize");
    context.uniform2fv_with_f32_array(canvas_size_uniform.as_ref(), variables.canvas_size.as_slice());  
}


fn upload_dynamic_uniforms(context: &WebGl2RenderingContext, program: &WebGlProgram, variables: &DynamicVariables) -> Result<(), JsValue> {
    let time_uniform = context.get_uniform_location(&program, "time");
    context.uniform1f(time_uniform.as_ref(), variables.time);
    
    let song_time_uniform = context.get_uniform_location(&program, "songTime");
    context.uniform1f(song_time_uniform.as_ref(), variables.song_position);

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
