#version 300 es

in vec4 position;
out vec3 out_position;
flat out int vertex_id;

void main() {
    gl_Position = position;
    out_position = position.xyz;
    vertex_id = gl_VertexID;
}