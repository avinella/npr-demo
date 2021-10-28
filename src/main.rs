use std::default;

use amethyst::{
    animation::*,
    assets::{
        AssetLoaderSystemData, AssetPrefab, AssetStorage, Completion, Handle, Loader, Prefab,
        PrefabData, PrefabLoader, PrefabLoaderSystemDesc, ProgressCounter, RonFormat,
    },
    controls::{ControlTagPrefab, FlyControlBundle, FlyControlTag},
    core::{Transform, TransformBundle},
    derive::PrefabData,
    ecs::{
        prelude::{Entity, World, WorldExt},
        ReadStorage, Write, WriteStorage,
    },
    input::{get_key, is_close_requested, is_key_down, InputBundle, StringBindings},
    prelude::*,
    renderer::{
        formats::mesh::ObjFormat,
        light::{Light, PointLight},
        palette::rgb::Rgb,
        plugins::{RenderPbr3D, RenderToWindow},
        rendy::mesh::{Normal, Position, Tangent, TexCoord},
        shape::Shape,
        types::DefaultBackend,
        Camera, ImageFormat, Material, MaterialDefaults, Mesh, RenderSkybox, RenderingBundle,
    },
    utils::{
        application_root_dir,
        auto_fov::{AutoFov, AutoFovSystem},
        scene::BasicScenePrefab,
        tag::{Tag, TagFinder},
    },
    window::DisplayConfig,
    winit::{ElementState, VirtualKeyCode},
    Application, Error, GameData, GameDataBuilder, SimpleState, StateData,
};
use amethyst_gltf::*;
use serde::{Deserialize, Serialize};

const CLEAR: [f32; 4] = [0.1, 0.1, 0.1, 1.0];
const WIN_WIDTH: f32 = 1024.0;
const WIN_HEIGHT: f32 = 768.0;

type MyPrefabData = (
    Option<BasicScenePrefab<(Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>)>>,
    Option<AnimationSetPrefab<AnimationId, Transform>>,
);

#[derive(Default, Deserialize, Serialize, PrefabData)]
#[serde(default)]
struct FoxPrefabData {
    gltf: Option<AssetPrefab<GltfSceneAsset, GltfSceneFormat>>,
    tag: Option<Tag<AnimationMarker>>
}

#[derive(Eq, PartialOrd, PartialEq, Hash, Debug, Copy, Clone, Deserialize, Serialize)]
enum AnimationId {
    Scale,
    Rotate,
    Translate,
    Test,
}

struct SphereAni {
    pub sphere: Option<Entity>,
    rate: f32,
    current_ani: AnimationId,
}

impl Default for SphereAni {
    fn default() -> Self {
        SphereAni {
            sphere: None,
            rate: 1.0,
            current_ani: AnimationId::Translate,
        }
    }
}

impl SimpleState for SphereAni {
    fn on_start(&mut self, state_data: StateData<'_, GameData<'_, '_>>) {
        let StateData { world, .. } = state_data;
        initialize_camera(world);
        //initialize_sphere(state_data.world);
        initialize_light(world);

        let prefab_handle = world.exec(|loader: PrefabLoader<'_, MyPrefabData>| {
            loader.load("sphere_animation.ron", RonFormat, ())
        });
        self.sphere = Some(world.create_entity().with(prefab_handle).build());

        let (animation_set, animation) = {
            let loader = world.read_resource::<Loader>();

            let sampler = loader.load_from_data(
                Sampler {
                    input: vec![0., 1.],
                    output: vec![
                        SamplerPrimitive::Vec3([0., 0., 0.]),
                        SamplerPrimitive::Vec3([0., 1., 0.]),
                    ],
                    function: InterpolationFunction::Step,
                },
                (),
                &world.read_resource(),
            );

            let animation = loader.load_from_data(
                Animation::new_single(0, TransformChannel::Translation, sampler),
                (),
                &world.read_resource(),
            );
            let mut animation_set: AnimationSet<AnimationId, Transform> = AnimationSet::new();
            animation_set.insert(AnimationId::Test, animation.clone());
            (animation_set, animation)
        };

        let entity = world
            .create_entity()
            .with(animation_set)
            .with(RestState::new(Transform::default()))
            .build();
        let mut storage = world.write_storage::<AnimationControlSet<AnimationId, Transform>>();
        let control_set = get_animation_set(&mut storage, entity).unwrap();
        control_set.add_animation(
            AnimationId::Test,
            &animation,
            EndControl::Loop(None),
            1.0,
            AnimationCommand::Start,
        );
    }

    fn handle_event(
        &mut self,
        state_data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        let StateData { world, .. } = state_data;
        if let StateEvent::Window(event) = &event {
            if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
                return Trans::Quit;
            }
            match get_key(&event) {
                Some((VirtualKeyCode::Space, ElementState::Pressed)) => {
                    add_animation(
                        world,
                        self.sphere.unwrap(),
                        self.current_ani,
                        self.rate,
                        None,
                        true,
                    );
                }

                Some((VirtualKeyCode::D, ElementState::Pressed)) => {
                    add_animation(
                        world,
                        self.sphere.unwrap(),
                        AnimationId::Translate,
                        self.rate,
                        None,
                        false,
                    );
                    add_animation(
                        world,
                        self.sphere.unwrap(),
                        AnimationId::Rotate,
                        self.rate,
                        Some((AnimationId::Translate, DeferStartRelation::End)),
                        false,
                    );
                    add_animation(
                        world,
                        self.sphere.unwrap(),
                        AnimationId::Scale,
                        self.rate,
                        Some((AnimationId::Rotate, DeferStartRelation::Start(0.666))),
                        false,
                    );
                }

                Some((VirtualKeyCode::Left, ElementState::Pressed)) => {
                    get_animation_set::<AnimationId, Transform>(
                        &mut world.write_storage(),
                        self.sphere.unwrap(),
                    )
                    .unwrap()
                    .step(self.current_ani, StepDirection::Backward);
                }

                Some((VirtualKeyCode::Right, ElementState::Pressed)) => {
                    get_animation_set::<AnimationId, Transform>(
                        &mut world.write_storage(),
                        self.sphere.unwrap(),
                    )
                    .unwrap()
                    .step(self.current_ani, StepDirection::Forward);
                }

                Some((VirtualKeyCode::F, ElementState::Pressed)) => {
                    self.rate = 1.0;
                    get_animation_set::<AnimationId, Transform>(
                        &mut world.write_storage(),
                        self.sphere.unwrap(),
                    )
                    .unwrap()
                    .set_rate(self.current_ani, self.rate);
                }

                Some((VirtualKeyCode::V, ElementState::Pressed)) => {
                    self.rate = 0.0;
                    get_animation_set::<AnimationId, Transform>(
                        &mut world.write_storage(),
                        self.sphere.unwrap(),
                    )
                    .unwrap()
                    .set_rate(self.current_ani, self.rate);
                }

                Some((VirtualKeyCode::H, ElementState::Pressed)) => {
                    self.rate = 0.5;
                    get_animation_set::<AnimationId, Transform>(
                        &mut world.write_storage(),
                        self.sphere.unwrap(),
                    )
                    .unwrap()
                    .set_rate(self.current_ani, self.rate);
                }

                Some((VirtualKeyCode::R, ElementState::Pressed)) => {
                    self.current_ani = AnimationId::Rotate;
                }

                Some((VirtualKeyCode::S, ElementState::Pressed)) => {
                    self.current_ani = AnimationId::Scale;
                }

                Some((VirtualKeyCode::T, ElementState::Pressed)) => {
                    self.current_ani = AnimationId::Translate;
                }

                _ => {}
            };
        }
        Trans::None
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct AnimationMarker;

#[derive(Default)]
struct AniObject {
    entity: Option<Entity>,
    initialized: bool,
    progress: Option<ProgressCounter>,
}

impl SimpleState for AniObject {
    fn on_start(&mut self, state_data: StateData<'_, GameData<'_, '_>>) {
        let StateData { world, .. } = state_data;
        initialize_camera(world);
        initialize_light(world);

        self.progress = Some(ProgressCounter::default());

        world.exec(
            |(loader, mut scene): (PrefabLoader<'_, FoxPrefabData>, Write<'_, Scene>)| {
                scene.handle = Some(loader.load(
                    "model_animation.ron",
                    RonFormat,
                    self.progress.as_mut().unwrap(),
                ));
            },
        );
    }

    fn update(&mut self, state_data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        if !self.initialized {
            let remove = match self.progress.as_ref().map(|p| p.complete()) {
                None | Some(Completion::Loading) => false,

                Some(Completion::Complete) => {
                    let scene_handle = state_data
                        .world
                        .read_resource::<Scene>()
                        .handle
                        .as_ref()
                        .unwrap()
                        .clone();

                    state_data.world.create_entity().with(scene_handle).build();

                    true
                }

                Some(Completion::Failed) => {
                    println!("Error: {:?}", self.progress.as_ref().unwrap().errors());
                    return Trans::Quit;
                }
            };
            if remove {
                self.progress = None;
            }
            if self.entity.is_none() {
                if let Some(entity) = state_data
                    .world
                    .exec(|finder: TagFinder<'_, AnimationMarker>| finder.find())
                {
                    self.entity = Some(entity);
                    self.initialized = true;
                }
            }
        }
        Trans::None
    }

    fn handle_event(
        &mut self,
        state_data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        let StateData { world, .. } = state_data;
        if let StateEvent::Window(event) = &event {
            if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
                return Trans::Quit;
            } else if is_key_down(&event, VirtualKeyCode::Space) {
                toggle_or_cycle_animation(
                    self.entity,
                    &mut world.write_resource(),
                    &world.read_storage(),
                    &mut world.write_storage(),
                );
                Trans::None
            } else {
                Trans::None
            }
        } else {
            Trans::None
        }
    }
}

#[derive(Default)]
struct Scene {
    handle: Option<Handle<Prefab<FoxPrefabData>>>,
    animation_index: usize,
}

// impl SimpleState for Scene {
//     fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
//         let StateData { world, .. } = data;

//         let mut transform = Transform::default();
//         transform.set_translation_xyz(0.0, 0.0, 0.0);

//         let material_defaults = world.read_resource::<MaterialDefaults>().0.clone();
//         let material = world.exec(|loader: AssetLoaderSystemData<'_, Material>| {
//             loader.load_from_data(
//                     Material {
//                         ..material_defaults
//                     },
//                     (),
//                 )
//             },
//         );

//         world
//             .create_entity()
//             .with(self.mesh_handle.clone())
//             .with(material)
//             //.with(transform)
//             .build();
//     }

//     fn handle_event(
//         &mut self,
//         state_data: StateData<'_, GameData<'_, '_>>,
//         event: StateEvent,
//     ) -> SimpleTrans {
//         let StateData { world, .. } = state_data;
//         if let StateEvent::Window(event) = &event {
//             if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
//                 return Trans::Quit;
//             }
//         }
//         Trans::None
//     }
// }

fn toggle_or_cycle_animation(
    entity: Option<Entity>,
    scene: &mut Scene,
    sets: &ReadStorage<'_, AnimationSet<usize, Transform>>,
    controls: &mut WriteStorage<'_, AnimationControlSet<usize, Transform>>,
) {
    if let Some((entity, Some(animations))) = entity.map(|entity| (entity, sets.get(entity))) {
        if animations.animations.len() > scene.animation_index {
            let animation = animations.animations.get(&scene.animation_index).unwrap();
            let set = get_animation_set::<usize, Transform>(controls, entity).unwrap();
            if set.has_animation(scene.animation_index) {
                set.toggle(scene.animation_index);
            } else {
                println!("Running animation {}", scene.animation_index);
                set.add_animation(
                    scene.animation_index,
                    animation,
                    EndControl::Normal,
                    1.0,
                    AnimationCommand::Start,
                );
            }
            scene.animation_index += 1;
            if scene.animation_index >= animations.animations.len() {
                scene.animation_index = 0;
            }
        }
    }
}

fn initialize_camera(world: &mut World) {
    let mut transform = Transform::default();
    transform.set_translation_xyz(0.0, 0.0, 10.0);

    world
        .create_entity()
        .with(Camera::standard_3d(WIN_WIDTH, WIN_HEIGHT))
        .with(transform)
        .with(FlyControlTag)
        .build();
}

fn initialize_light(world: &mut World) {
    let light: Light = PointLight {
        intensity: 10.0,
        color: Rgb::new(1.0, 1.0, 1.0),
        ..PointLight::default()
    }
    .into();

    let mut transform = Transform::default();
    transform.set_translation_xyz(5.0, 5.0, 20.0);

    world.create_entity().with(light).with(transform).build();
}

fn initialize_sphere(world: &mut World) {
    let mesh = world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
        loader.load_from_data(
            Shape::Sphere(100, 100)
                .generate::<(Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>)>(None)
                .into(),
            (),
        )
    });

    let material_defaults = world.read_resource::<MaterialDefaults>().0.clone();
    let material = world.exec(|loader: AssetLoaderSystemData<'_, Material>| {
        loader.load_from_data(
            Material {
                ..material_defaults
            },
            (),
        )
    });

    let mut transform = Transform::default();
    transform.set_translation_xyz(0.0, 0.0, 0.0);

    world
        .create_entity()
        .with(mesh)
        .with(material)
        .with(transform)
        .build();
}

fn add_animation(
    world: &mut World,
    entity: Entity,
    id: AnimationId,
    rate: f32,
    defer: Option<(AnimationId, DeferStartRelation)>,
    toggle_if_exists: bool,
) {
    let animation = world
        .read_storage::<AnimationSet<AnimationId, Transform>>()
        .get(entity)
        .and_then(|s| s.get(&id))
        .cloned()
        .unwrap();
    let mut sets = world.write_storage();
    let control_set = get_animation_set::<AnimationId, Transform>(&mut sets, entity).unwrap();
    match defer {
        None => {
            if toggle_if_exists && control_set.has_animation(id) {
                control_set.toggle(id);
            } else {
                control_set.add_animation(
                    id,
                    &animation,
                    EndControl::Normal,
                    rate,
                    AnimationCommand::Start,
                );
            }
        }

        Some((defer_id, defer_relation)) => {
            control_set.add_deferred_animation(
                id,
                &animation,
                EndControl::Normal,
                rate,
                AnimationCommand::Start,
                defer_id,
                defer_relation,
            );
        }
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::Logger::from_config(amethyst::LoggerConfig {
        level_filter: amethyst::LogLevelFilter::Error,
        ..Default::default()
    })
    .start();

    let app_root = application_root_dir()?;
    let assets_dir = app_root.join("assets/");
    let key_bindings_path = app_root.join("config/input.ron");

    let display_config = DisplayConfig {
        title: "NPR".to_string(),
        dimensions: Some((WIN_WIDTH as u32, WIN_HEIGHT as u32)),
        ..Default::default()
    };

    // for sphere
    // let scene_data = GameDataBuilder::default()
    //     .with_system_desc(PrefabLoaderSystemDesc::<MyPrefabData>::default(), "", &[])
    //     .with_bundle(AnimationBundle::<AnimationId, Transform>::new(
    //         "animation_control_system",
    //         "sampler_interpolation_system",
    //     ))?
    //     .with_bundle(
    //         FlyControlBundle::<StringBindings>::new(
    //             Some(String::from("move_x")),
    //             Some(String::from("move_y")),
    //             Some(String::from("move_z")),
    //         )
    //         .with_sensitivity(0.1, 0.1),
    //     )?
    //     .with_bundle(
    //         TransformBundle::new().with_dep(&["fly_movement", "sampler_interpolation_system"]),
    //     )?
    //     .with_bundle(
    //         InputBundle::<StringBindings>::new().with_bindings_from_file(&key_bindings_path)?,
    //     )?
    //     .with_bundle(
    //         RenderingBundle::<DefaultBackend>::new()
    //             .with_plugin(RenderToWindow::from_config(display_config).with_clear(CLEAR))
    //             .with_plugin(RenderPbr3D::default()),
    //     )?;

    let anim_data = GameDataBuilder::default()
        .with(AutoFovSystem::default(), "auto_fov", &[])
        .with_system_desc(
            PrefabLoaderSystemDesc::<FoxPrefabData>::default(),
            "scene_loader",
            &[],
        )
        .with_system_desc(
            GltfSceneLoaderSystemDesc::default(),
            "gltf_loader",
            &["scene_loader"], // This is important so that entity instantiation is performed in a single frame.
        )
        .with_bundle(
            AnimationBundle::<usize, Transform>::new("animation_control", "sampler_interpolation")
                .with_dep(&["gltf_loader"]),
        )?
        .with_bundle(
            FlyControlBundle::<StringBindings>::new(
                Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z")),
            )
            .with_sensitivity(0.1, 0.1)
            .with_speed(50.),
        )?
        .with_bundle(TransformBundle::new().with_dep(&[
            "animation_control",
            "sampler_interpolation",
            "fly_movement",
        ]))?
        .with_bundle(VertexSkinningBundle::new().with_dep(&[
            "transform_system",
            "animation_control",
            "sampler_interpolation",
        ]))?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(RenderToWindow::from_config(display_config).with_clear(CLEAR))
                .with_plugin(RenderPbr3D::default().with_skinning())
                .with_plugin(RenderSkybox::default()),
        )?
        .with_bundle(
            InputBundle::<StringBindings>::new().with_bindings_from_file(&key_bindings_path)?,
        )?;

    let state: AniObject = Default::default();
    let mut scene = Application::new(assets_dir, state, anim_data)?;
    scene.run();

    Ok(())
}
