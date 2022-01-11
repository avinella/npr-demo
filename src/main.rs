// use npr_app::custom_render::RenderCustom;

use amethyst::{Application, Error, GameData, GameDataBuilder, SimpleState, StateData, animation::*, assets::{
        AssetLoaderSystemData, AssetPrefab, AssetStorage, Completion, Handle, Loader, Prefab,
        PrefabData, PrefabLoader, PrefabLoaderSystemDesc, ProgressCounter, RonFormat,
    }, controls::{ControlTagPrefab, FlyControlBundle, FlyControlTag}, core::{Transform, TransformBundle}, derive::PrefabData, ecs::{
        prelude::{Entity, World, WorldExt},
        ReadStorage, Write, WriteStorage,
    }, input::{get_key, is_close_requested, is_key_down, InputBundle, StringBindings}, prelude::*, renderer::{Camera, ImageFormat, Material, MaterialDefaults, Mesh, RenderDebugLines, RenderShaded3D, RenderSkybox, RenderingBundle, camera::CameraPrefab, formats::mesh::ObjFormat, light::{Light, LightPrefab, PointLight}, palette::rgb::Rgb, plugins::{RenderPbr3D, RenderToWindow}, rendy::mesh::{Normal, Position, Tangent, TexCoord}, shape::Shape, types::DefaultBackend}, utils::{
        application_root_dir,
        auto_fov::{AutoFov, AutoFovSystem},
        scene::BasicScenePrefab,
        tag::{Tag, TagFinder},
    }, window::DisplayConfig, winit::{ElementState, VirtualKeyCode}};
use amethyst_gltf::*;
use serde::{Deserialize, Serialize};
use npr_app::custom_render::RenderCustom3D;

const CLEAR: [f32; 4] = [0.1, 0.1, 0.1, 1.0];
const WIN_WIDTH: f32 = 1024.0;
const WIN_HEIGHT: f32 = 768.0;

#[derive(Default, Deserialize, Serialize, PrefabData)]
#[serde(default)]
struct AnimationPrefabData {
    gltf: Option<AssetPrefab<GltfSceneAsset, GltfSceneFormat>>,
    tag: Option<Tag<AnimationMarker>>,
}

#[derive(Default, Deserialize, Serialize, PrefabData)]
#[serde(default)]
struct ScenePrefabData {
    transform: Option<Transform>,
    light: Option<LightPrefab>,
}

#[derive(Clone, Serialize, Deserialize)]
struct AnimationMarker;

#[derive(Default)]
struct Scene {
    anim_handle: Option<Handle<Prefab<AnimationPrefabData>>>,
    animation_index: usize,
}
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

        self.progress = Some(ProgressCounter::default());

        world.exec(
            |(loader, mut scene): (PrefabLoader<'_, AnimationPrefabData>, Write<'_, Scene>)| {
                scene.anim_handle = Some(loader.load(
                    "prefabs/model_animation.ron",
                    RonFormat,
                    self.progress.as_mut().unwrap(),
                ));
            },
        );
        let scene_handle = world.exec(|loader: PrefabLoader<'_, ScenePrefabData>| {
            loader.load("prefabs/scene.ron", RonFormat, ())
        });
        world.create_entity().with(scene_handle).build();
    }

    fn update(&mut self, state_data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        if !self.initialized {
            let remove = match self.progress.as_ref().map(|p| p.complete()) {
                None | Some(Completion::Loading) => false,

                Some(Completion::Complete) => {
                    let scene_handle = state_data
                        .world
                        .read_resource::<Scene>()
                        .anim_handle
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

            // end the last animation before starting new one
            // model needs to be in rest state before starting an animation
            let mut last_animation = animations.animations.len() - 1;
            if scene.animation_index != 0 {
                last_animation = scene.animation_index - 1;
            }
            set.abort(last_animation);

            if set.has_animation(scene.animation_index) {
                set.toggle(scene.animation_index);
            } else {
                println!("Running animation {}", scene.animation_index);
                set.add_animation(
                    scene.animation_index,
                    animation,
                    EndControl::Loop(None),
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
    transform.set_translation_xyz(0.0, 5.0, 30.0);

    world
        .create_entity()
        .with(Camera::standard_3d(WIN_WIDTH, WIN_HEIGHT))
        .with(transform)
        .with(FlyControlTag)
        .build();
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
        title: "NPR Demo".to_string(),
        dimensions: Some((WIN_WIDTH as u32, WIN_HEIGHT as u32)),
        ..Default::default()
    };

    let anim_data = GameDataBuilder::default()
        .with(AutoFovSystem::default(), "auto_fov", &[])
        .with_system_desc(
            PrefabLoaderSystemDesc::<AnimationPrefabData>::default(),
            "anim_loader",
            &[],
        )
        .with_system_desc(
            PrefabLoaderSystemDesc::<ScenePrefabData>::default(),
            "",
            &[],
        )
        .with_system_desc(
            GltfSceneLoaderSystemDesc::default(),
            "gltf_loader",
            &["anim_loader"],
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
                .with_plugin(RenderCustom3D::default().with_skinning())
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
