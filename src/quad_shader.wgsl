struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance: u32,
};

struct VertexOutput {
    @location(0) world_position: vec2<f32>,
    @location(1) texture_coordinate: vec2<f32>,
    @location(2) color: vec3<f32>,
    @builtin(position) clip_position: vec4<f32>,
};

struct Camera {
    position: vec2<f32>,
    rotation: f32,
    zoom: f32,
    screen_size: vec2<f32>,
};

@group(0)
@binding(0)
var<uniform> camera: Camera;

struct Quad {
    position: vec2<f32>,
    scale: vec2<f32>,
    color: vec3<f32>,
    rotation: f32,
};

struct QuadStorageBuffer {
    quad_count: u32,
    quads: array<Quad>,
};

@group(1)
@binding(0)
var<storage> quad_buffer: QuadStorageBuffer;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    let uv = vec2<f32>(
        f32((model.vertex_index >> 0u) & 1u),
        f32((model.vertex_index >> 1u) & 1u),
    );

    let quad = quad_buffer.quads[model.instance];

    output.world_position = uv - 0.5;
    output.world_position *= quad.scale;
    output.world_position = vec2<f32>(
        output.world_position.x * cos(-quad.rotation) - output.world_position.y * sin(-quad.rotation),
        output.world_position.y * cos(-quad.rotation) + output.world_position.x * sin(-quad.rotation),
    );
    output.world_position += quad.position;

    output.texture_coordinate = uv;
    output.color = quad.color;

    let aspect = camera.screen_size.x / camera.screen_size.y;
    let camera_relative_position = output.world_position - camera.position;
    let camera_zoom_position = camera_relative_position * camera.zoom;
    let camera_rotated_position = vec2<f32>(
        camera_zoom_position.x * cos(camera.rotation) - camera_zoom_position.y * sin(camera.rotation),
        camera_zoom_position.y * cos(camera.rotation) + camera_zoom_position.x * sin(camera.rotation),
    );

    output.clip_position = vec4<f32>(
        camera_rotated_position.x / aspect,
        camera_rotated_position.y,
        0.0,
        1.0,
    );
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
