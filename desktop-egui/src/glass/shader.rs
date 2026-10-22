//! One GLSL program draws the window backdrop and every glass surface (egui_glow paint callbacks).

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
uniform int u_kind;      // 0 backdrop, 1 glass
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
float sd(vec2 p, vec2 b, float r) {
  vec2 q = abs(p) - b + r;
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
  vec2 p = gl_FragCoord.xy;
  if (u_kind == 0) { o = vec4(bg(p), 1.0); return; }
  vec2 ctr = u_rect.xy + u_rect.zw * 0.5;
  vec2 hs = u_rect.zw * 0.5;
  float r = min(u_radius, min(hs.x, hs.y));
  vec2 q = p - ctr;
  float s = sd(q, hs, r);
  float aa = 1.0 - smoothstep(-0.5, 0.5, s);
  if (aa <= 0.0) discard;
  float d = -s;
  vec2 n = normalize(vec2(
    sd(q + vec2(1., 0.), hs, r) - sd(q - vec2(1., 0.), hs, r),
    sd(q + vec2(0., 1.), hs, r) - sd(q - vec2(0., 1.), hs, r)) + 1e-5);
  float edge = 22.0 * u_dpi;
  float f = pow(clamp(1.0 - d / edge, 0.0, 1.0), 2.0);
  vec2 sp = p + n * f * edge * 0.9;
  vec3 col = bg(sp);
  float rad = 16.0 * u_dpi;
  for (int i = 0; i < 6; i++) {
    float a = 6.2831853 * float(i) / 6.0;
    col += bg(sp + vec2(cos(a), sin(a)) * rad * 0.5);
    col += bg(sp + vec2(cos(a + 0.5), sin(a + 0.5)) * rad);
  }
  col /= 13.0;
  col = mix(vec3(dot(col, vec3(0.299, 0.587, 0.114))), col, 1.8);
  float t = clamp(0.5 * ((p.x - u_rect.x) / u_rect.z + 1.0 - (p.y - u_rect.y) / u_rect.w), 0.0, 1.0);
  col = mix(col, vec3(1.0), clamp(mix(0.22, 0.08, t) * u_k, 0.0, 0.9));
  col = mix(col, vec3(1.0), (1.0 - smoothstep(0.0, 1.3 * u_dpi, d)) * 0.30);
  float v = (p.y - u_rect.y) / u_rect.w;
  col = mix(col, vec3(1.0), (1.0 - smoothstep(1.0 * u_dpi, 2.4 * u_dpi, d)) * smoothstep(0.7, 1.0, v) * 0.55);
  o = vec4(col * aa, aa);
}
";

pub struct Program {
    program: glow::Program,
    vao: glow::VertexArray,
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
        Program { program, vao: gl.create_vertex_array().expect("vao") }
    };
    cc.egui_ctx.data_mut(|d| d.insert_temp(Id::NULL.with("glass-program"), Arc::new(Mutex::new(p))));
}

/// The window backdrop over `rect` (the whole window). `glow`: 1 midnight glow, 0 pitch black.
pub fn backdrop(ctx: &Context, rect: Rect, glow: f32) -> PaintCallback {
    callback(ctx, rect, 0, 0.0, 1.0, glow)
}

/// A glass surface over `rect`: corner `radius` in points, fill strength `k` (1 = the design's glass).
pub fn surface(ctx: &Context, rect: Rect, radius: f32, k: f32, glow: f32) -> PaintCallback {
    callback(ctx, rect, 1, radius, k, glow)
}

fn callback(ctx: &Context, rect: Rect, kind: i32, radius: f32, k: f32, glow_amount: f32) -> PaintCallback {
    let program: Arc<Mutex<Program>> = ctx.data(|d| d.get_temp(Id::NULL.with("glass-program"))).expect("glass::install was not called");
    PaintCallback {
        rect,
        callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
            use glow::HasContext as _;
            let gl = painter.gl();
            let p = program.lock().unwrap();
            let vp = info.viewport_in_pixels();
            let [sw, sh] = info.screen_size_px;
            // SAFETY: draws with the program made in `install`, on egui's own context.
            unsafe {
                gl.use_program(Some(p.program));
                let u = |n: &str| gl.get_uniform_location(p.program, n);
                gl.uniform_2_f32(u("u_screen").as_ref(), sw as f32, sh as f32);
                gl.uniform_4_f32(u("u_rect").as_ref(), vp.left_px as f32, vp.from_bottom_px as f32, vp.width_px as f32, vp.height_px as f32);
                gl.uniform_1_f32(u("u_radius").as_ref(), radius * info.pixels_per_point);
                gl.uniform_1_f32(u("u_dpi").as_ref(), info.pixels_per_point);
                gl.uniform_1_f32(u("u_k").as_ref(), k);
                gl.uniform_1_f32(u("u_glow").as_ref(), glow_amount);
                gl.uniform_1_i32(u("u_kind").as_ref(), kind);
                gl.bind_vertex_array(Some(p.vao));
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            }
        })),
    }
}
