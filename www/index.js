import * as wasm from "spotfish";
import SpotifyWebApi from "spotify-web-api-node";
import SpotifyWebApiServer from 'spotify-web-api-node/src/server-methods';
SpotifyWebApi._addMethods(SpotifyWebApiServer);

const canvas = document.getElementById("canvas");
canvas.width = window.innerWidth;
canvas.height = window.innerHeight;

const clientId = '055277caad62422e96b0b985d48752cb';

const VERT_SHADER = `#version 300 es
 
in vec4 position;

void main() {

    gl_Position = position;
}`;

const FRAG_SHADER = `#version 300 es
 
precision highp float;
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
uniform int numSections;
uniform uint currentSongSection;

uniform SongSegments
{   
    SongSegment songSegments[100];
};
uniform int numSegments;

out vec4 outColor;

void main() {
    vec2 pos = gl_FragCoord.xy / canvasSize;
    SongSection section = songSections[currentSongSection];
    float beatsPerSecond = 60.0 / section.tempo;
    float beatProgress = pow(mod(songTime, beatsPerSecond) / beatsPerSecond, 2.0);
    float intenseness = pow(2.0, -10.0 * (1.0 - ((section.loudness + 60.0) / 60.0)));
    vec2 beatPos = vec2((1.0+sin(2.0*PI*(0.5/beatsPerSecond)*songTime))/2.0, 0.5);
    float dist = 1.0 - pow(length(abs(pos - beatPos)), 1.1);

    vec4 intensenessColor = vec4(intenseness, 1.0-intenseness, 0.0, 1.0);
    vec4 uvColor = vec4(pos.x, pos.y, mix(pos.x, pos.y, sin(beatProgress * section.tempo / 60.0)), 1.0);
    vec4 beatColor = vec4(intenseness, 1.0 - intenseness, beatProgress, 1.0);

    outColor = mix(intensenessColor, mix(uvColor, beatColor, dist), 2.0 - dist);
}`;


let instance = new wasm.Instance(canvas, VERT_SHADER, FRAG_SHADER);
let startRender = () => {
    function draw() {
        instance.draw();
        requestAnimationFrame(draw);
    }

    requestAnimationFrame(draw);
};

if (window.location.hash.includes("access_token")) {
    const params = new URLSearchParams(window.location.hash.substring(1));
    const accessToken = params.get("access_token");
    const spotifyApi = new SpotifyWebApi({
        clientId: clientId
    });

    spotifyApi.setAccessToken(accessToken);

    spotifyApi.getMyCurrentPlaybackState().then(state => {
        const playbackTimestamp = new Date();
        if (state != null) {
            if (state.body.item?.id) {
                spotifyApi.getAudioAnalysisForTrack(state.body.item.id).then(analysis => {
                    console.log(analysis.body);
                    const playbackOffset = (new Date() - playbackTimestamp);
                    console.log("Set song", instance.set_song(analysis.body.sections, analysis.body.segments, (state.body.progress_ms + playbackOffset) / 1000));
                    startRender();
                });
            }
        }
    });
} else {
    startRender();
    document.body.addEventListener("click",
    () => {
        const scopes = ['user-read-private', 'user-read-playback-state'],
          redirectUri = 'http://localhost:8080',
          state = 'start',
          showDialog = true,
          responseType = 'token';
    
        const spotifyApi = new SpotifyWebApi({
          redirectUri: redirectUri,
          clientId: clientId
        });
        
        const authorizeURL = spotifyApi.createAuthorizeURL(
          scopes,
          state,
          showDialog,
          responseType
        );
        
        window.open(authorizeURL, "_self");
    });
}
