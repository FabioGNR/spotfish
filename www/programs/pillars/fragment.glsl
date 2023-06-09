#version 300 es
 
precision highp float;
precision highp int;

layout(std140) uniform;
#define PI 3.1415926538
in vec3 out_position;
flat in int vertex_id;

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

out vec4 outColor;

void main() {
    vec2 pos = gl_FragCoord.xy / canvasSize;
    if (out_position.y < 0.01) {
        pos = (out_position.xz + 10.0) / 20.0;
    }
    SongSection section = songSections[currentSongSection];
    float sectionPos = songTime - section.start;

    float beatsPerSecond = 60.0 / section.tempo;
    float beatProgress = pow(mod(sectionPos, beatsPerSecond) / beatsPerSecond, 2.0);
    float intenseness = pow(2.0, -10.0 * (1.0 - ((section.loudness + 60.0) / 60.0)));

    vec2 beatPos = vec2((1.0+sin(2.0*PI*(0.5/beatsPerSecond)*sectionPos))/2.0, 0.5);
    float dist = 1.0 - pow(length(abs(pos - beatPos)), 1.1);

    vec4 intensenessColor = vec4(intenseness, 1.0-intenseness, 0.0, 1.0);
    vec4 uvColor = vec4(pos.x, pos.y, mix(pos.x, pos.y, sin(beatProgress * section.tempo / 60.0)), 1.0);
    vec4 beatColor = vec4(intenseness, 1.0 - intenseness, beatProgress, 1.0);

    SongSegment segment = songSegments[currentSongSegment];
    SongSegment nextSegment = songSegments[min( numSegments - 1u, currentSongSegment + 1u)];
    float segmentProgress = max(0.0, (songTime - (segment.start + segment.loudness_max_time)) / segment.duration);
    float pitchWidth = 1.0 / 11.0;
    float pitchPadding = 0.001;
    float pitchColor = segment.timbre[1][1] / segment.timbre[0][1];

    outColor = mix(intensenessColor, mix(uvColor, beatColor, dist), 2.0 - dist) * 0.2;
    outColor.w = 1.0;

    for (int i = 0; i < 12; i++) {
        float pitch = segment.pitches[i / 4][i % 4];
        float nextPitch = nextSegment.pitches[i / 4][i % 4];
        float interpolatedPitch = pow(mix(pitch, nextPitch, pow(segmentProgress, 2.0)), 2.0);
        if (abs(pos.x-pitchWidth/2.0 - pitchWidth*float(i)) < (pitchWidth/2.0-pitchPadding) && pos.y < interpolatedPitch && mod(pos.y, 0.01) > 0.001) {
            outColor = vec4(pos.x, 1.0-pos.x, pitchColor, 1.0);
        }
    }

    if (out_position.y > 0.01) {
        outColor = vec4(out_position.y / 14.0, beatsPerSecond, out_position.z / 20.0, 1.0);
    } else {
        // outColor = vec4(out_position.z, 1.0, 1.0, 1.0);
    }
}