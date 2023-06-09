#version 300 es
precision highp float;
precision highp int;
layout(std140) uniform;
#define PI 3.1415926538


struct SongSection {
    float start;
    float duration;
    float loudness;
    float tempo;
};

struct SongSegment {
    float start;
    float duration;
    float loudness_max_time;
    vec4 pitches[3];
    vec4 timbre[3];
};

uniform vec2 canvasSize;
uniform float time;
uniform float songTime;
uniform SongSections
{   
    SongSection songSections[64];
};
uniform uint numSections;
uniform uint currentSongSection;

uniform SongSegments
{   
    SongSegment songSegments[100];
};
uniform uint numSegments;
uniform uint currentSongSegment;


in vec4 position;
out vec3 out_position;
flat out int vertex_id;

void main() {
    float aspect = 16.0/9.0;
    float fovRad = 80.0 * PI / 180.0;
    float fov = tan(PI * 0.5 - 0.5*fovRad);
    float far = 2000.0;
    float near = 0.05;
    float rangeInv = 1.0 / (near-far);

    mat4 perspective = mat4(fov/aspect, 0.0, 0.0, 0.0,
        0.0, fov, 0.0, 0.0,
        0.0, 0.0, (near+far) * rangeInv, -1.0,
        0.0, 0.0, near * far * rangeInv * 2.0, 0.0
    );

    SongSection section = songSections[currentSongSection];
    float sectionPos = songTime - section.start;
    float beatsPerSecond = 60.0 / section.tempo;
    float beatProgress = pow(mod(sectionPos, beatsPerSecond) / beatsPerSecond, 2.0);

    vec3 up = vec3(0.0, 1.0, 0.0);
    vec3 target = vec3(1.0, 0.5, sin(sectionPos) * -5.0);
    vec3 cameraPos = vec3(target.x, target.y+0.1, target.z + 3.000001);
    vec3 zAxis = normalize(target - cameraPos);
    vec3 xAxis = normalize(cross(up, zAxis));
    vec3 yAxis = normalize(cross(zAxis, xAxis));

    mat4 lookAt = mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        cameraPos.xyz, 1.0
    ) * mat4(
            xAxis.x, yAxis.x, zAxis.x, 0.0,
            xAxis.y, yAxis.y, zAxis.y, 0.0,
            xAxis.z, yAxis.z, zAxis.z, 0.0,
            0.0, 0.0, 0.0, 1.0
        );

    vec4 pos = vec4(position.x, position.y * sin(1.0/beatsPerSecond), position.z , 1.0);
    gl_Position = perspective * inverse(lookAt) * pos;
    out_position = pos.xyz;
    vertex_id = gl_VertexID;
}