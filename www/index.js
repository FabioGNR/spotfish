import * as wasm from "spotfish";
import SpotifyWebApi from "spotify-web-api-node";
import SpotifyWebApiServer from 'spotify-web-api-node/src/server-methods';
SpotifyWebApi._addMethods(SpotifyWebApiServer);

const canvas = document.getElementById("canvas");
const clientId = '055277caad62422e96b0b985d48752cb';

console.log(window.location);
if (window.location.hash.includes("access_token")) {
    const params = new URLSearchParams(window.location.hash.substring(1));
    const accessToken = params.get("access_token");
    const spotifyApi = new SpotifyWebApi({
        clientId: clientId
    });

    spotifyApi.setAccessToken(accessToken);

    spotifyApi.getMyCurrentPlaybackState().then(state => {
        if (state != null) {
            console.log(state.body);
            if (state.body.item?.id) {
                spotifyApi.getAudioFeaturesForTrack(state.body.item.id).then(features => {
                    console.log(features.body)
                    spotifyApi.getAudioAnalysisForTrack(state.body.item.id).then(analysis => {
                        console.log(analysis.body);
                    });
                });
            }
        }
    });
} else {
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
