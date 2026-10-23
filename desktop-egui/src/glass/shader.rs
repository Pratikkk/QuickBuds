//! One GLSL program draws the window (a rounded, transparent-cornered pane) and every glass surface (egui_glow paint
//! callbacks). A glass surface is real glass: before it draws it copies what is already on screen behind it, blurs
//! that, bends it at the edge, and adds only a light fill, a rim and a highlight.

use eframe::{egui, egui_glow, glow};
use egui::{Context, Id, PaintCallback, Rect};
use std::sync::{Arc, Mutex};

const VERT: &str = "
const vec2 v[4] = vec2[4](vec2(-1.,-1.), vec2(1.,-1.), vec2(-1.,1.), vec2(1.,1.));
void main() { gl_Position = vec4(v[gl_VertexID], 0., 1.); }
";

const FRAG: &str = "
precision highp float;
uniform vec2 u_screen;
uniform vec4 u_rect;     // x, y from bottom, w, h (px)
uniform float u_radius;  // px
uniform float u_dpi;
uniform float u_k;       // fill strength
uniform float u_glow;    // 0 black window, 1 midnight glow
uniform float u_alpha;   // the widget's opacity (fades, entrances)
uniform vec4 u_solid;    // window: rgb of a solid colour when w > 0.5 (the other styles), else the glow
uniform vec4 u_edge;     // window: the edge line colour
uniform vec4 u_clip;     // glass: the copied region (min xy, max xy, px)
uniform sampler2D u_back;
uniform int u_kind;      // 0 window, 1 glass
out vec4 o;

vec3 blob(vec3 c, vec2 uv, vec2 ctr, vec3 col, float a) {
  vec2 d = (uv - ctr) * vec2(u_screen.x / u_screen.y, 1.0);
  float f = a * (1.0 - smoothstep(0.0, 0.62, length(d)));
  return mix(c, col, f);
}
vec3 bg(vec2 p) {
  vec2 uv = p / u_screen;
  vec3 c = mix(vec3(0.0), vec3(0.039, 0.039, 0.086), u_glow);
  c = blob(c, uv, vec2(0.12, 0.82), vec3(0.290, 0.427, 1.0), 0.24 * u_glow);
  c = blob(c, uv, vec2(0.88, 0.88), vec3(0.761, 0.482, 1.0), 0.17 * u_glow);
  c = blob(c, uv, vec2(0.78, 0.08), vec3(1.0, 0.616, 0.420), 0.13 * u_glow);
  c = blob(c, uv, vec2(0.08, 0.05), vec3(0.153, 0.816, 0.769), 0.15 * u_glow);
  return c;
}
vec3 back(vec2 p) {
  p = clamp(p, u_clip.xy, u_clip.zw);
  return texture(u_back, p / u_screen).rgb;
}
float sd(vec2 p, vec2 b, float r) {
  vec2 q = abs(p) - b + r;
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
  vec2 p = gl_FragCoord.xy;
  vec2 ctr = u_rect.xy + u_rect.zw * 0.5;
  vec2 hs = u_rect.zw * 0.5;
  float r = min(u_radius, min(hs.x, hs.y));
  vec2 q = p - ctr;
  float s = sd(q, hs, r);
  float aa = 1.0 - smoothstep(-0.5, 0.5, s);
  if (aa <= 0.0) discard;
  float d = -s;
  if (u_kind == 0) {
    // The window: its own background inside a rounded edge, with a light line on the edge.
    vec3 c = u_solid.w > 0.5 ? u_solid.rgb : bg(p);
    float line = (1.0 - smoothstep(0.0, 1.4 * u_dpi, d)) * u_edge.a;
    c = mix(c, u_edge.rgb, line);
    if (u_solid.w < 0.5) {
      float v = (p.y - u_rect.y) / u_rect.w;
      c = mix(c, vec3(1.0), (1.0 - smoothstep(1.0 * u_dpi, 3.0 * u_dpi, d)) * smoothstep(0.85, 1.0, v) * 0.35);
    }
    o = vec4(c * aa, aa);
    return;
  }
  vec2 n = normalize(vec2(
    sd(q + vec2(1., 0.), hs, r) - sd(q - vec2(1., 0.), hs, r),
    sd(q + vec2(0., 1.), hs, r) - sd(q - vec2(0., 1.), hs, r)) + 1e-5);
  // lens: what is behind bends near the edge
  float edge = 22.0 * u_dpi;
  float f = pow(clamp(1.0 - d / edge, 0.0, 1.0), 2.0);
  vec2 sp = p + n * f * edge * 0.9;
  // blur: 48 taps on a golden-angle spiral, a gaussian weight, the spiral turned per pixel so no pattern shows
  float rad = 16.0 * u_dpi;
  float turn = 6.2831853 * fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715))));
  vec3 col = vec3(0.0);
  float wsum = 0.0;
  for (int i = 0; i < 48; i++) {
    float r01 = (float(i) + 0.5) / 48.0;
    float a = 2.3999632 * float(i) + turn;
    float w = exp(-2.0 * r01 * r01);
    col += back(sp + vec2(cos(a), sin(a)) * rad * sqrt(r01)) * w;
    wsum += w;
  }
  col /= wsum;
  col = mix(vec3(dot(col, vec3(0.299, 0.587, 0.114))), col, 1.8);
  float t = clamp(0.5 * ((p.x - u_rect.x) / u_rect.z + 1.0 - (p.y - u_rect.y) / u_rect.w), 0.0, 1.0);
  // Only a light veil: the black behind a button shows through it.
  col = mix(col, vec3(1.0), clamp(mix(0.11, 0.035, t) * u_k, 0.0, 0.9));
  col = mix(col, vec3(1.0), (1.0 - smoothstep(0.0, 1.3 * u_dpi, d)) * 0.30);
  float v = (p.y - u_rect.y) / u_rect.w;
  col = mix(col, vec3(1.0), (1.0 - smoothstep(1.0 * u_dpi, 2.4 * u_dpi, d)) * smoothstep(0.7, 1.0, v) * 0.55);
  o = vec4(col * aa, aa) * u_alpha;
}
";

pub struct Program {
    program: glow::Program,
    vao: glow::VertexArray,
    /// The copy of the screen behind a glass surface: (texture, width, height).
    tex: Option<(glow::Texture, i32, i32)>,
}

/// Compiles the program and keeps it in the egui context. Call once from the app's constructor.
pub fn install(cc: &eframe::CreationContext<'_>) {
    use glow::HasContext as _;
    let gl = cc.gl.as_ref().expect("the glow backend");
    // SAFETY: plain GL object creation on the context eframe made current.
    let p = unsafe {
        let program = gl.create_program().expect("program");
        let shaders: Vec<_> = [(glow::VERTEX_SHADER, VERT), (glow::FRAGMENT_SHADER, FRAG)].iter().map(|(ty, src)| {
            let sh = gl.create_shader(*ty).expect("shader");
            gl.shader_source(sh, &format!("#version 330\n{src}"));
            gl.compile_shader(sh);
            assert!(gl.get_shader_compile_status(sh), "shader: {}", gl.get_shader_info_log(sh));
            gl.attach_shader(program, sh);
            sh
        }).collect();
        gl.link_program(program);
        assert!(gl.get_program_link_status(program), "{}", gl.get_program_info_log(program));
        for sh in shaders { gl.detach_shader(program, sh); gl.delete_shader(sh); }
        Program { program, vao: gl.create_vertex_array().expect("vao"), tex: None }
    };
    cc.egui_ctx.data_mut(|d| d.insert_temp(Id::NULL.with("glass-program"), Arc::new(Mutex::new(p))));
}

/// What the window draws behind everything.
#[derive(Clone, Copy)]
pub struct Window {
    /// Corner radius in points (0 when maximized).
    pub radius: f32,
    /// 1 midnight glow, 0 pitch black (Glass only).
    pub glow: f32,
    /// A solid colour for the other styles.
    pub solid: Option<[f32; 3]>,
    /// The line along the edge: rgb and strength.
    pub edge: [f32; 4],
}

/// The window pane over `rect` (the whole window).
pub fn window(ctx: &Context, rect: Rect, w: Window) -> PaintCallback {
    callback(ctx, rect, 0, w.radius, 1.0, w.glow, 1.0, w.solid.map_or([0.0; 4], |c| [c[0], c[1], c[2], 1.0]), w.edge)
}

/// A glass surface over `rect`: corner `radius` in points, fill strength `k` (1 = the design's glass).
pub fn surface(ctx: &Context, rect: Rect, radius: f32, k: f32, alpha: f32) -> PaintCallback {
    callback(ctx, rect, 1, radius, k, 0.0, alpha, [0.0; 4], [0.0; 4])
}

#[allow(clippy::too_many_arguments)]
fn callback(ctx: &Context, rect: Rect, kind: i32, radius: f32, k: f32, glow_amount: f32, alpha: f32, solid: [f32; 4], edge: [f32; 4]) -> PaintCallback {
    let program: Arc<Mutex<Program>> = ctx.data(|d| d.get_temp(Id::NULL.with("glass-program"))).expect("glass::install was not called");
    PaintCallback {
        rect,
        callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
            use glow::HasContext as _;
            let gl = painter.gl();
            let mut p = program.lock().unwrap();
            let vp = info.viewport_in_pixels();
            let [sw, sh] = info.screen_size_px;
            let (sw, sh) = (sw as i32, sh as i32);
            let margin = (40.0 * info.pixels_per_point) as i32;
            // SAFETY: draws with the program made in `install`, on egui's own context; the copy reads the window's framebuffer.
            unsafe {
                gl.use_program(Some(p.program));
                let prog = p.program;
                let u = |n: &str| gl.get_uniform_location(prog, n);
                if kind == 1 {
                    // The screen behind the surface, with a margin for the blur and the lens.
                    if p.tex.map_or(true, |t| t.1 != sw || t.2 != sh) {
                        if let Some(t) = p.tex.take() { gl.delete_texture(t.0); }
                        let tex = gl.create_texture().expect("texture");
                        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, sw, sh, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
                        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                        p.tex = Some((tex, sw, sh));
                    }
                    let x0 = (vp.left_px - margin).max(0);
                    let y0 = (vp.from_bottom_px - margin).max(0);
                    let x1 = (vp.left_px + vp.width_px + margin).min(sw);
                    let y1 = (vp.from_bottom_px + vp.height_px + margin).min(sh);
                    gl.active_texture(glow::TEXTURE0);
                    gl.bind_texture(glow::TEXTURE_2D, p.tex.map(|t| t.0));
                    if x1 > x0 && y1 > y0 {
                        gl.copy_tex_sub_image_2d(glow::TEXTURE_2D, 0, x0, y0, x0, y0, x1 - x0, y1 - y0);
                    }
                    gl.uniform_4_f32(u("u_clip").as_ref(), x0 as f32 + 0.5, y0 as f32 + 0.5, x1 as f32 - 0.5, y1 as f32 - 0.5);
                    gl.uniform_1_i32(u("u_back").as_ref(), 0);
                }
                gl.uniform_2_f32(u("u_screen").as_ref(), sw as f32, sh as f32);
                gl.uniform_4_f32(u("u_rect").as_ref(), vp.left_px as f32, vp.from_bottom_px as f32, vp.width_px as f32, vp.height_px as f32);
                gl.uniform_1_f32(u("u_radius").as_ref(), radius * info.pixels_per_point);
                gl.uniform_1_f32(u("u_dpi").as_ref(), info.pixels_per_point);
                gl.uniform_1_f32(u("u_k").as_ref(), k);
                gl.uniform_1_f32(u("u_glow").as_ref(), glow_amount);
                gl.uniform_1_f32(u("u_alpha").as_ref(), alpha);
                gl.uniform_4_f32(u("u_solid").as_ref(), solid[0], solid[1], solid[2], solid[3]);
                gl.uniform_4_f32(u("u_edge").as_ref(), edge[0], edge[1], edge[2], edge[3]);
                gl.uniform_1_i32(u("u_kind").as_ref(), kind);
                gl.bind_vertex_array(Some(p.vao));
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            }
        })),
    }
}
