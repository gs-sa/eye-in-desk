import { AmbientLight, BoxGeometry, Color, DirectionalLight, Euler, GridHelper, Group, Material, Matrix4, Mesh, MeshBasicMaterial, MeshMatcapMaterial, MeshPhongMaterial, MeshStandardMaterial, MeshToonMaterial, PerspectiveCamera, Scene, TextureLoader, WebGLRenderer } from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls";
import { OBJLoader } from 'three/examples/jsm/loaders/OBJLoader';
import { URDFRobot } from 'urdf-js/src/URDFClasses';
import URDFLoader from 'urdf-js/src/URDFLoader';

interface Object {
    id: number,
    x: number,
    y: number,
    z: number,
    rot: number,
}

interface objectsResp {
    objects: string[]
}

interface Control {
    rotate_left: number,
    rotate_up: number,
}

export class State {
    renderer: WebGLRenderer
    scene: Scene
    camera: PerspectiveCamera
    robot: URDFRobot
    orbitControls: OrbitControls
    jointsWs: WebSocket
    primitiveWs: WebSocket
    controlsWs: WebSocket
    obj_map: Map<number, Group>
    constructor(renderer: WebGLRenderer,
        scene: Scene,
        camera: PerspectiveCamera,
        robot: URDFRobot,
        orbitControls: OrbitControls,
        obj_map: Map<number, Group>
    ) {
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

            obj_map.forEach((v, _) => {
                v.visible = false;
            });
            for (const obj of objs) {
                let g = this.obj_map.get(obj.id);
                if (g !== undefined) {
                    g.position.set(obj.x, obj.y, obj.z);
                    g.rotation.set(0, 0, obj.rot);
                    g.visible = true;
                }
            }
            this.scene.children[0] = g;
        }
        this.orbitControls = orbitControls;
        this.controlsWs = new WebSocket("ws://localhost:8000/controlsWs");
        this.controlsWs.onmessage = (ev: MessageEvent<string>) => {
            let control: Control = JSON.parse(ev.data);
            dispatchEvent(new PointerEvent("pointerdown", {
                pointerId: 1,
                pointerType: "mouse",
                clientX: 0,
                clientY: 0,
            }));
            dispatchEvent(new PointerEvent("pointermove", {
                pointerId: 1,
                pointerType: "mouse",
                clientX: -control.rotate_left,
                clientY: -control.rotate_up,
            }));
            dispatchEvent(new PointerEvent("pointerup", {
                pointerId: 1,
                pointerType: "mouse",
            }));
        };
        this.obj_map = obj_map;
    }
    render() {
        requestAnimationFrame(this.render.bind(this));
        this.renderer.render(this.scene, this.camera);
    }
}

export async function initState(container: HTMLDivElement) {
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

    // camera.lookAt(0.64 - 0.225, 0.3, 0);

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

    // add ycbs
    let objects = await fetch("./ycb/objects.json").then((resp) => {
        return resp.json()
    }).then((objects: objectsResp) => {
        return objects.objects;
    });

    let obj_loader = new OBJLoader();
    let texture_loader = new TextureLoader();
    let obj_map = new Map<number, Group>();
    let ycb_group = new Group();
    ycb_group.rotateX(-Math.PI / 2);
    scene.add(ycb_group);
    for (let i = 0; i < objects.length; i++) {
        const obj = objects[i];
        let resp = await fetch(`./ycb/${obj}/google_16k/textured.obj`);
        if (resp.ok) {
            let text = await resp.text();
            let g = obj_loader.parse(text);

            let mesh = g.children[0] as Mesh;
            texture_loader.load(`./ycb/${obj}/google_16k/texture_map.png`, (t) => {
                (mesh.material as MeshPhongMaterial).map = t;
            });

            obj_map.set(i, g);
            g.visible = false;
            ycb_group.add(g);
        }
    }
    // let g = (obj_map.get(2) as Group);
    // g.visible = true;
    // g.position.set(0.5, 0.2, 0);
    // g.rotateZ(Math.PI / 2);

    let oc = new OrbitControls(camera, renderer.domElement);
    oc.target.set(0.5, 0, 0);
    camera.position.set(0.5, 1.5, 1.5);
    oc.update();
    // urdf loader
    let loader = new URDFLoader();
    loader.load("./Panda/panda.urdf", (robot) => {
        robot.rotation.set(-90 / 360 * 2 * Math.PI, 0, 0);
        scene.add(robot);
        state = new State(renderer, scene, camera, robot, oc, obj_map);
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