use crate::{data, render};
use render::{
    BindingInfo, Buffer, Context, Framebuffer, Indices, Uniform, UniformValue, VertexBuffer,
};

use web_sys::WebGl2RenderingContext as GL;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

static LINE_VERT_SHADER: &'static str = include_str!("./shaders/line.vert");
static LINE_FRAG_SHADER: &'static str = include_str!("./shaders/line.frag");
static TEXTURE_VERT_SHADER: &'static str = include_str!("./shaders/texture.vert");
static TEXTURE_FRAG_SHADER: &'static str = include_str!("./shaders/texture.frag");

pub struct Drawer {
    context: Context,

    width: u32,
    height: u32,

    grid_width: u32,
    grid_height: u32,
    line_count: u32,

    draw_lines_pass: render::RenderPass,
    draw_texture_pass: render::RenderPass,
}

impl Drawer {
    pub fn new(
        context: &Context,
        width: u32,
        height: u32,
        grid_width: u32,
        grid_height: u32,
    ) -> Result<Self> {
        let line_count = grid_width * grid_height;

        let line_vertices = Buffer::from_f32(
            &context,
            &data::LINE_VERTICES.to_vec(),
            GL::ARRAY_BUFFER,
            GL::STATIC_DRAW,
        )?;
        let line_indices = Buffer::from_u16(
            &context,
            &data::LINE_INDICES.to_vec(),
            GL::ELEMENT_ARRAY_BUFFER,
            GL::STATIC_DRAW,
        )?;
        let basepoints = Buffer::from_f32(
            &context,
            &data::new_points(20, 20),
            GL::ARRAY_BUFFER,
            GL::STREAM_DRAW, // TODO: what’s the most appropriate type here?
        )?;
        let color_data: [f32; 16] = [
            0.14509804, 0.68627451, 0.80784314, 0.0, //
            0.14509804, 0.68627451, 0.80784314, 1.0, //
            0.14509804, 0.68627451, 0.80784314, 1.0, //
            0.14509804, 0.68627451, 0.80784314, 0.0, //
        ];
        let colors = Buffer::from_f32(
            &context,
            &color_data.to_vec(),
            GL::ARRAY_BUFFER,
            GL::STATIC_DRAW,
        )?;

        let plane_vertices = Buffer::from_f32(
            &context,
            &data::PLANE_VERTICES.to_vec(),
            GL::ARRAY_BUFFER,
            GL::STATIC_DRAW,
        )
        .unwrap();
        let plane_indices = Buffer::from_u16(
            &context,
            &data::PLANE_INDICES.to_vec(),
            GL::ELEMENT_ARRAY_BUFFER,
            GL::STATIC_DRAW,
        )
        .unwrap();
        let draw_lines_program =
            render::Program::new(&context, (LINE_VERT_SHADER, LINE_FRAG_SHADER))?;
        let draw_texture_program =
            render::Program::new(&context, (TEXTURE_VERT_SHADER, TEXTURE_FRAG_SHADER))?;

        let draw_lines_pass = render::RenderPass::new(
            &context,
            vec![
                VertexBuffer {
                    buffer: line_vertices,
                    binding: BindingInfo {
                        name: "position".to_string(),
                        size: 3,
                        type_: GL::FLOAT,
                        ..Default::default()
                    },
                },
                VertexBuffer {
                    buffer: basepoints,
                    binding: BindingInfo {
                        name: "basepoint".to_string(),
                        size: 3,
                        type_: GL::FLOAT,
                        divisor: 1,
                        ..Default::default()
                    },
                },
                VertexBuffer {
                    buffer: colors,
                    binding: BindingInfo {
                        name: "color".to_string(),
                        size: 4,
                        type_: GL::FLOAT,
                        ..Default::default()
                    },
                },
            ],
            Indices::IndexBuffer {
                buffer: line_indices,
                primitive: GL::TRIANGLES,
            },
            draw_lines_program,
        )
        .unwrap();
        let draw_texture_pass = render::RenderPass::new(
            &context,
            vec![VertexBuffer {
                buffer: plane_vertices,
                binding: BindingInfo {
                    name: "position".to_string(),
                    size: 3,
                    type_: GL::FLOAT,
                    ..Default::default()
                },
            }],
            Indices::IndexBuffer {
                buffer: plane_indices,
                primitive: GL::TRIANGLES,
            },
            draw_texture_program,
        )
        .unwrap();

        Ok(Self {
            context: context.clone(),
            width,
            height,
            grid_width: grid_width,
            grid_height: grid_height,
            line_count: line_count,

            draw_lines_pass,
            draw_texture_pass,
        })
    }

    pub fn draw_lines(&self, timestep: f32, texture: &Framebuffer) -> Result<()> {
        self.context
            .viewport(0, 0, self.width as i32, self.height as i32);

        self.context.enable(GL::BLEND);
        self.context.blend_func_separate(
            GL::SRC_ALPHA,
            GL::ONE_MINUS_SRC_ALPHA,
            GL::ONE,
            GL::ONE_MINUS_SRC_ALPHA,
        );

        self.draw_lines_pass.draw(
            vec![
                Uniform {
                    name: "deltaT".to_string(),
                    value: UniformValue::Float(timestep),
                },
                Uniform {
                    name: "velocityTexture".to_string(),
                    value: UniformValue::Texture2D(&texture.texture, 0),
                },
            ],
            20 * 20,

    pub fn draw_texture(&self, texture: &Framebuffer) -> Result<()> {
        self.context
            .viewport(0, 0, self.width as i32, self.height as i32);

        self.draw_texture_pass.draw(
            vec![Uniform {
                name: "inputTexture".to_string(),
                value: UniformValue::Texture2D(&texture.texture, 0),
            }],
            1,
        )
    }
}
