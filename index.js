
const rust = import('./pkg/volumetric_shading_webgl');
var cubeStrip = [
    1, 1, 0,
    0, 1, 0,
    1, 1, 1,
    0, 1, 1,
    0, 0, 1,
    0, 1, 0,
    0, 0, 0,
    1, 1, 0,
    1, 0, 0,
    1, 1, 1,
    1, 0, 1,
    0, 0, 1,
    1, 0, 0,
    0, 0, 0
];

var takeScreenShot = false;
var canvas = null;

var gl = null;
var shader = null;
var programReady = null;
var volumeTexture = null;
var colormapTex = null;
var fileRegex = /(\w+)_(\d+)x(\d+)x(\d+)_(\w+)\.*/;
var proj = null;
var camera = null;
var projView = null;
var tabFocused = true;
var newVolumeUpload = true;
var targetFrameTime = 32;
var samplingRate = 1.0;
var WIDTH = 640;
var HEIGHT = 480;

//const defaultEye = vec3.set(vec3.create(), 0.5, 0.5, 1.5);
//const center = vec3.set(vec3.create(), 0.5, 0.5, 0.5);
//const up = vec3.set(vec3.create(), 0.0, 1.0, 0.0);

var volumes = {
    "Skull": "skull_256x256x256_uint8.raw",
};

var colormaps = {
    "Cool Warm": "colormaps/cool-warm-paraview.png",
};

const FPS_THROTTLE= 33;
glDraw = null;
function drawStep() {
    const currTime = Date.now();
    if(programReady){
        let elapsedTime = currTime - lastDrawTime;
        if(elapsedTime >= FPS_THROTTLE) {
            lastDrawTime = currTime;
            programReady.render_from_state()
        }
    }
}

var lastDrawTime = Date.now();
window.onload = function(){
    rust.then(m => {
        canvas = document.getElementById("rustCanvas");
//        gl = canvas.getContext("webgl2");
//        if (!gl) {
//            alert("Unable to initialize WebGL2. Your browser may not support it");
//            return;
//        }
//        console.log(gl);


        glDraw = new m.GlDraw(canvas);
        // Load the default colormap and upload it, after which we
        // load the default volume.
        var colormapImage = new Image();
        colormapImage.onload = function() {

                file = "skull_256x256x256_uint8.raw"
                var volDims = [256, 256, 256];

                var url = "data/" + file;
                var req = new XMLHttpRequest();


                req.open("GET", url, true);
                req.responseType = "arraybuffer";
                req.onload = function(evt) {
                    var dataBuffer = req.response;
                    dataBuffer = new Uint8Array(dataBuffer);
                    programReady = glDraw.setup_program(dataBuffer);

                };
                req.send();
        };
        colormapImage.src = "colormaps/cool-warm-paraview.png";
        setInterval(drawStep, 33);

    })

}



//var Shader = function(gl, vertexSrc, fragmentSrc) {
//    var self = this;
//    this.program = compileShader(gl, vertexSrc, fragmentSrc);
//
//}

// Compile and link the shaders vert and frag. vert and frag should contain
// the shader source code for the vertex and fragment shaders respectively
// Returns the compiled and linked program, or null if compilation or linking failed
var compileShader = function(gl, vert, frag){

    var program = gl.createProgram();
    return program;
}

var getGLExtension = function(ext) {
    if (!gl.getExtension(ext)) {
        alert("Missing " + ext + " WebGL extension");
        return false;
    }
    return true;
}

