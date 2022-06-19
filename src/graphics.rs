// I used this guide for learning OpenGL in Rust: https://rust-tutorials.github.io/learn-opengl/introduction.html
use glutin::{Context, PossiblyCurrent};
use gl33::*;

type GLuint = u32;

type Vertex = [f32; 3];
type TriIndexes = [u32; 3];

const VERTICES: [Vertex; 4] =
    [[0.5, 0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]];

const INDICES: [TriIndexes; 2] = [[0, 1, 3], [1, 2, 3]];

const VERT_SHADER: &str = r#"#version 330 core
        layout (location = 0) in vec3 pos;
        void main() {
            gl_Position = vec4(pos.x, pos.y, pos.z, 1.0);
        }
        "#;

const FRAG_SHADER: &str = r#"#version 330 core
        out vec4 final_color;

        void main() {
            final_color = vec4(1.0, 0.5, 0.2, 1.0);
        }
"#;

pub struct VertexArray(pub GLuint);
impl VertexArray {
    /// Creates a new vertex array object
    pub fn new(ctx: &GL) -> Option<Self> {
        let mut vao = 0;
        unsafe { ctx.gl.GenVertexArrays(1, &mut vao) };
        if vao != 0 {
            Some(Self(vao))
        } else {
            None
        }
    }

    /// Bind this vertex array as the current vertex array object
    pub fn bind(&self, ctx: &GL) {
        unsafe { ctx.gl.BindVertexArray(self.0) }
    }

    /// Clear the current vertex array object binding.
    pub fn clear_binding(ctx: &GL) {
        unsafe { ctx.gl.BindVertexArray(0) }
    }
}

pub enum BufferType {
    Array,
    ElementArray,
}

impl BufferType {
    pub fn glenum(&self) -> GLenum {
        use BufferType::*;
        match self {
            Array => GL_ARRAY_BUFFER,
            ElementArray => GL_ELEMENT_ARRAY_BUFFER
        }
    }
}

pub struct Buffer(pub GLuint, pub BufferType);

impl Buffer {
    /// Initialize a new buffer object
    pub fn new(ctx: &GL, buffer_type: BufferType) -> Option<Self> {
        let mut bo = 0;
        unsafe { ctx.gl.GenBuffers(1, &mut bo); }
        if bo != 0 {
            Some(Self(bo, buffer_type))
        } else {
            None
        }
    }
    /// Bind this buffer to the GL context
    pub fn bind(&self, ctx: &GL) {
        unsafe { ctx.gl.BindBuffer(self.1.glenum(), self.0) }
    }
    /// Clear the specified buffer type from the GL buffer binding.
    pub fn clear_binding(ctx: &GL, buffer_type: BufferType) {
        unsafe { ctx.gl.BindBuffer(buffer_type.glenum(), 0) }
    }
}

/// Places a slice of data into a previously-bound buffer.
pub fn buffer_data(ctx: &GL, ty: BufferType, data: &[u8], usage: GLenum) {
    unsafe {
        ctx.gl.BufferData(
            ty.glenum(),
            data.len().try_into().unwrap(),
            data.as_ptr().cast(),
            usage,
        );
    }
}

/// The types of shader object.
pub enum ShaderType {
    /// Vertex shaders determine the position of geometry within the screen.
    Vertex,
    /// Fragment shaders determine the color output of geometry.
    ///
    /// Also other values, but mostly color.
    Fragment,
}

impl ShaderType {
    pub fn glenum(&self) -> GLenum {
        use ShaderType::*;
        match self {
            Vertex => GL_VERTEX_SHADER,
            Fragment => GL_FRAGMENT_SHADER
        }
    }
}

/// A handle to a [Shader
/// Object](https://www.khronos.org/opengl/wiki/GLSL_Object#Shader_objects)
pub struct Shader(pub GLuint);

impl Shader {
    /// Makes a new shader.
    ///
    /// Prefer the [`Shader::from_source`](Shader::from_source) method.
    ///
    /// Possibly skip the direct creation of the shader object and use
    /// [`ShaderProgram::from_vert_frag`](ShaderProgram::from_vert_frag).
    pub fn new(ctx: &GL, ty: ShaderType) -> Option<Self> {
        let shader = unsafe { ctx.gl.CreateShader(ty.glenum()) };
        if shader != 0 {
            Some(Self(shader))
        } else {
            None
        }
    }

    /// Assigns a source string to the shader.
    ///
    /// Replaces any previously assigned source.
    pub fn set_source(&self, ctx: &GL, src: &str) {
        unsafe {
            ctx.gl.ShaderSource(
                self.0,
                1,
                &(src.as_bytes().as_ptr().cast()),
                &(src.len().try_into().unwrap()),
            );
        }
    }

    /// Compiles the shader based on the current source.
    pub fn compile(&self, ctx: &GL) {
        unsafe { ctx.gl.CompileShader(self.0) };
    }

    /// Checks if the last compile was successful or not.
    pub fn compile_success(&self, ctx: &GL) -> bool {
        let mut compiled = 0;
        unsafe { ctx.gl.GetShaderiv(self.0, GL_COMPILE_STATUS, &mut compiled) };
        compiled != 0
    }

    /// Gets the info log for the shader.
    ///
    /// Usually you use this to get the compilation log when a compile failed.
    pub fn info_log(&self, ctx: &GL) -> String {
        let mut needed_len = 0;
        unsafe { ctx.gl.GetShaderiv(self.0, GL_INFO_LOG_LENGTH, &mut needed_len) };
        let mut v: Vec<u8> = Vec::with_capacity(needed_len.try_into().unwrap());
        let mut len_written = 0_i32;
        unsafe {
            ctx.gl.GetShaderInfoLog(
                self.0,
                v.capacity().try_into().unwrap(),
                &mut len_written,
                v.as_mut_ptr().cast(),
            );
            v.set_len(len_written.try_into().unwrap());
        }
        String::from_utf8_lossy(&v).into_owned()
    }

    /// Marks a shader for deletion.
    ///
    /// Note: This _does not_ immediately delete the shader. It only marks it for
    /// deletion. If the shader has been previously attached to a program then the
    /// shader will stay allocated until it's unattached from that program.
    pub fn delete(self, ctx: &GL) {
        unsafe { ctx.gl.DeleteShader(self.0) };
    }

    /// Takes a shader type and source string and produces either the compiled
    /// shader or an error message.
    ///
    /// Prefer [`ShaderProgram::from_vert_frag`](ShaderProgram::from_vert_frag),
    /// it makes a complete program from the vertex and fragment sources all at
    /// once.
    pub fn from_source(ctx: &GL, ty: ShaderType, source: &str) -> Result<Self, String> {
        let id = Self::new(ctx, ty)
            .ok_or_else(|| "Couldn't allocate new shader".to_string())?;
        id.set_source(ctx, source);
        id.compile(ctx);
        if id.compile_success(ctx) {
            Ok(id)
        } else {
            let out = id.info_log(ctx);
            id.delete(ctx);
            Err(out)
        }
    }
}

/// A handle to a [Program
/// Object](https://www.khronos.org/opengl/wiki/GLSL_Object#Program_objects)
pub struct ShaderProgram(pub GLuint);
impl ShaderProgram {
    /// Allocates a new program object.
    ///
    /// Prefer [`ShaderProgram::from_vert_frag`](ShaderProgram::from_vert_frag),
    /// it makes a complete program from the vertex and fragment sources all at
    /// once.
    pub fn new(ctx: &GL) -> Option<Self> {
        let prog = unsafe { ctx.gl.CreateProgram() };
        if prog != 0 {
            Some(Self(prog))
        } else {
            None
        }
    }

    /// Attaches a shader object to this program object.
    pub fn attach_shader(&self, ctx: &GL, shader: &Shader) {
        unsafe { ctx.gl.AttachShader(self.0, shader.0) };
    }

    /// Links the various attached, compiled shader objects into a usable program.
    pub fn link_program(&self, ctx: &GL) {
        unsafe { ctx.gl.LinkProgram(self.0) };
    }

    /// Checks if the last linking operation was successful.
    pub fn link_success(&self, ctx: &GL) -> bool {
        let mut success = 0;
        unsafe { ctx.gl.GetProgramiv(self.0, GL_LINK_STATUS, &mut success) };
        success != 0
    }

    /// Gets the log data for this program.
    ///
    /// This is usually used to check the message when a program failed to link.
    pub fn info_log(&self, ctx: &GL) -> String {
        let mut needed_len = 0;
        unsafe { ctx.gl.GetProgramiv(self.0, GL_INFO_LOG_LENGTH, &mut needed_len) };
        let mut v: Vec<u8> = Vec::with_capacity(needed_len.try_into().unwrap());
        let mut len_written = 0_i32;
        unsafe {
            ctx.gl.GetProgramInfoLog(
                self.0,
                v.capacity().try_into().unwrap(),
                &mut len_written,
                v.as_mut_ptr().cast(),
            );
            v.set_len(len_written.try_into().unwrap());
        }
        String::from_utf8_lossy(&v).into_owned()
    }

    /// Sets the program as the program to use when drawing.
    pub fn use_program(&self, ctx: &GL) {
        unsafe { ctx.gl.UseProgram(self.0) };
    }

    /// Marks the program for deletion.
    ///
    /// Note: This _does not_ immediately delete the program. If the program is
    /// currently in use it won't be deleted until it's not the active program.
    /// When a program is finally deleted and attached shaders are unattached.
    pub fn delete(self, ctx: &GL) {
        unsafe { ctx.gl.DeleteProgram(self.0) };
    }

    /// Takes a vertex shader source string and a fragment shader source string
    /// and either gets you a working program object or gets you an error message.
    ///
    /// This is the preferred way to create a simple shader program in the common
    /// case. It's just less error prone than doing all the steps yourself.
    pub fn from_vert_frag(ctx: &GL, vert: &str, frag: &str) -> Result<Self, String> {
        let p =
            Self::new(ctx).ok_or_else(|| "Couldn't allocate a program".to_string())?;
        let v = Shader::from_source(ctx, ShaderType::Vertex, vert)
            .map_err(|e| format!("Vertex Compile Error: {}", e))?;
        let f = Shader::from_source(ctx, ShaderType::Fragment, frag)
            .map_err(|e| format!("Fragment Compile Error: {}", e))?;
        p.attach_shader(ctx, &v);
        p.attach_shader(ctx, &f);
        p.link_program(ctx);
        v.delete(ctx);
        f.delete(ctx);
        if p.link_success(ctx) {
            Ok(p)
        } else {
            let out = format!("Program Link Error: {}", p.info_log(ctx));
            p.delete(ctx);
            Err(out)
        }
    }
}

// OpenGL wrapper
pub struct GL {
    pub gl: GlFns
}

impl GL {
    pub fn new(ctx: &Context<PossiblyCurrent>) -> Self {
        let gl = unsafe {
            GlFns::load_from(&|p| {
                let c_str = std::ffi::CStr::from_ptr(p.cast());
                let rust_str = c_str.to_str().unwrap();
                ctx.get_proc_address(rust_str) as _
            })
                .unwrap()
        };

        Self { gl }
    }

    pub fn clear(&self) {
        unsafe { self.gl.Clear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT); }
    }

    pub fn clear_color(&self, r: f32, g: f32, b: f32, a: f32) {
        unsafe { self.gl.ClearColor(r, g, b, a); }
    }

    pub fn setup(&self) {
        self.clear_color(0.1, 0.1, 0.1, 1.0);
        let vao = VertexArray::new(self).unwrap();
        vao.bind(self);
        let vbo = Buffer::new(self, BufferType::Array).unwrap();
        vbo.bind(self);
        let ebo = Buffer::new(self, BufferType::ElementArray).unwrap();
        ebo.bind(self);
        buffer_data(
            self,
            BufferType::ElementArray,
            bytemuck::cast_slice(&INDICES),
            GL_STATIC_DRAW,
        );

        buffer_data(self, BufferType::Array, bytemuck::cast_slice(&VERTICES), GL_STATIC_DRAW);

        unsafe {
            self.gl.VertexAttribPointer(
                0,
                3,
                GL_FLOAT,
                false as u8,
                core::mem::size_of::<Vertex>().try_into().unwrap(),
                0 as *const _,
            );
            self.gl.EnableVertexAttribArray(0);
        }
        let shader_program =
            ShaderProgram::from_vert_frag(self, VERT_SHADER, FRAG_SHADER).unwrap();
        shader_program.use_program(self);
    }

    pub fn draw_frame(&self) {
        unsafe {
            self.clear();
            self.gl.DrawElements(GL_TRIANGLES, 6, GL_UNSIGNED_INT, 0 as *const _);
        }
    }
}