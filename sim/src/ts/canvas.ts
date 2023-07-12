import { AmbientLight, BoxGeometry, Color, DirectionalLight, GridHelper, Group, Mesh, MeshBasicMaterial, MeshMatcapMaterial, MeshStandardMaterial, MeshToonMaterial, PerspectiveCamera, Scene, WebGLRenderer } from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls";
import { URDFRobot } from 'urdf-js/src/URDFClasses';
import URDFLoader from 'urdf-js/src/URDFLoader';

interface Object {
    id: number,
    pos: number[],
    rot: number[],
}

export class State {
    renderer: WebGLRenderer
    scene: Scene
    camera: PerspectiveCamera
    robot: URDFRobot
    jointsWs: WebSocket
    primitiveWs: WebSocket
    constructor(renderer: WebGLRenderer,
        scene: Scene,
        camera: PerspectiveCamera,
        robot: URDFRobot) {
        this.renderer = renderer;
        this.scene = scene;
        this.camera = camera;
        this.robot = robot;
        this.jointsWs = new WebSocket("ws://localhost:8000/jointsWs");
        this.jointsWs.onmessage = (ev: MessageEvent<Blob>) => {
            ev.data.arrayBuffer().then((v) => {
                let joints = new Float64Array(v);
                this.robot.setAngles({
                    "panda_joint1": joints[0],
                    "panda_joint2": joints[1],
                    "panda_joint3": joints[2],
                    "panda_joint4": joints[3],
                    "panda_joint5": joints[4],
                    "panda_joint6": joints[5],
                    "panda_joint7": joints[6],
                })
            })
        }
        
        this.primitiveWs = new WebSocket("ws://localhost:8000/primitiveWs");
        this.primitiveWs.onmessage = (ev: MessageEvent<string>) => {
            let objs: Object[] = JSON.parse(ev.data);
            let g = new Group();
            if (objs.length !== 0) {
                let b = new Mesh(new BoxGeometry(
                    0.1, 0.1, 0.1
                ), new MeshMatcapMaterial({
                    color: "red"
                }));
                b.translateX(0.5);
                g.add(b);
            }
            for (const obj of objs) {
                
            }
            this.scene.children[0] = g;
        }
    }
    render() {
        requestAnimationFrame(this.render.bind(this));
        this.renderer.render(this.scene, this.camera);
    }
}

export function initState(container: HTMLDivElement) {
    // create renderer
    let renderer = new WebGLRenderer({
        // use input canvas
        // antialias
        antialias: true
    })
    // basic settings
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.setSize(window.innerWidth, window.innerHeight);
    renderer.physicallyCorrectLights = true;
    container.appendChild(renderer.domElement);

    // create scene
    let scene = new Scene();
    // background color = gray
    scene.background = new Color(0x3C3C3C);

    // add a empty group
    scene.add(new Group());

    // Perspective Camera & settings
    let camera = new PerspectiveCamera(50, window.innerWidth / window.innerHeight, 0.001, 100);
    camera.position.set(0.64 - 0.225, 1.5, 1.);
    camera.lookAt(0.64 - 0.225, 0.3, 0);

    // AmbientLight
    let l1 = new AmbientLight(0xFFFFFF, 1);

    // DirectionalLight
    let l2 = new DirectionalLight(0xFFFFFF, 1);
    l2.position.set(0, 1.5, 1);

    // add lights to scene
    scene.add(l1);
    scene.add(l2);

    // add grid
    let grid = new GridHelper(10, 100);
    scene.add(grid);

    // add desk
    // let desk = new BoxGeometry(1.28, 0.03, 0.64);
    // scene.add(new Mesh(desk, new MeshStandardMaterial({ color: new Color(0x8899DD) })));

    // add debug helpers
    // new OrbitControls(camera, renderer.domElement);

    // urdf loader
    let loader = new URDFLoader();
    loader.load("./Panda/panda.urdf", (robot) => {
        // robot.traverse((object)=>{
        //     if (object instanceof Mesh) {
        //         object.material = new MeshBasicMaterial({
        //             color: new Color(0xFFFFFF),
        //         });
        //     }
        // });

        robot.rotation.set(-90 / 360 * 2 * Math.PI, 0, 0);
        scene.add(robot);
        state = new State(renderer, scene, camera, robot);
        state.render();
    }, () => { }, () => { console.log("error") }, { packages: "./Panda" });
}

export let state: State | null = null;

window.onresize = function () {
    if (state !== null) {
        (state as State).camera.aspect = window.innerWidth / window.innerHeight;
        (state as State).camera.updateProjectionMatrix();
        (state as State).renderer.setSize(window.innerWidth, window.innerHeight);
    }
};