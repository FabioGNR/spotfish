import * as wasm from "spotfish";
import SpotifyWebApi from "spotify-web-api-node";
import SpotifyWebApiServer from 'spotify-web-api-node/src/server-methods';
import program from './programs/pillars/pillars';

SpotifyWebApi._addMethods(SpotifyWebApiServer);

const canvas = document.getElementById("canvas");
const canvasRect = canvas.getBoundingClientRect();
canvas.width = canvasRect.width;
canvas.height = canvasRect.height;

const clientId = '055277caad62422e96b0b985d48752cb';

const getVertex = fetch(program.vertexShader).then(response => response.text());
const getFragment = fetch(program.fragmentShader).then(response => response.text());

Promise.all([getVertex, getFragment]).then(([vertexShader, fragmentShader]) => {
    let instance = new wasm.Instance(canvas, vertexShader, fragmentShader, program.vertices, program.vertsPerPoly);
    window.addEventListener("keydown", () => {
        instance.print_song_time();
    });
    
    let startRender = () => {
        function draw() {
            instance.draw();
            if (document.visibilityState !== 'hidden')
                requestAnimationFrame(draw);
        }
    
        requestAnimationFrame(draw);
    };
    
    function startAuth() {
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
    }
    
    if (window.location.hash.includes("access_token")) {
        const params = new URLSearchParams(window.location.hash.substring(1));
        const accessToken = params.get("access_token");
        const spotifyApi = new SpotifyWebApi({
            clientId: clientId
        });
    
        spotifyApi.setAccessToken(accessToken);
    
        let currentSong = null;
    
        const updateSong = async () => {
            const state = await spotifyApi.getMyCurrentPlaybackState();
            const playbackTimestamp = new Date();
            if (state != null) {
                let song = state.body?.item?.id;
                if (song) {
                    return spotifyApi.getAudioAnalysisForTrack(state.body.item.id).then(analysis => {
                        if (currentSong != song) {
                            console.log(analysis.body);
                            currentSong = song;
                        }
                        const playbackOffset = (new Date() - playbackTimestamp);
                        instance.set_song(analysis.body.sections, analysis.body.segments, (state.body.progress_ms + playbackOffset) / 1000);
                    });
                }
            }
        };
    
        updateSong().then(() => {
            startRender();
            let updateInterval = undefined;
            updateInterval = setInterval(() => {
                try {
                    updateSong();
                } catch (error) {
                    console.log(error);
                    clearInterval(updateInterval);
                    startAuth();
                }
            }, 1500);
        })
        .catch((error) => {
            console.log(error);
            startAuth();
        });
    } else {
        startRender();
        document.body.addEventListener("click",
        () => {
            startAuth();
        });
    }
});
