//! Displays a 2D GLTF scene

extern crate amethyst;
extern crate amethyst_gltf;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

use amethyst::animation::{get_animation_set, AnimationBundle, AnimationCommand,
                          AnimationControlSet, AnimationSet, EndControl, VertexSkinningBundle};
use amethyst::assets::{AssetPrefab, Completion, Handle, Prefab, PrefabData, PrefabLoader,
                       PrefabLoaderSystem, ProgressCounter, RonFormat};
use amethyst::core::transform::{GlobalTransform, Transform, TransformBundle};
use amethyst::ecs::error::Error;
use amethyst::ecs::prelude::{Entity, ReadStorage, Write, WriteStorage};
use amethyst::input::{is_close_requested, is_key};
use amethyst::prelude::*;
use amethyst::renderer::*;
use amethyst::utils::tag::{Tag, TagFinder};
use amethyst_gltf::{GltfSceneAsset, GltfSceneFormat, GltfSceneLoaderSystem};

#[derive(Default)]
struct Example {
    entity: Option<Entity>,
    initialised: bool,
    progress: Option<ProgressCounter>,
}

#[derive(Clone, Serialize, Deserialize)]
struct AnimationMarker;

#[derive(Default)]
struct Scene {
    handle: Option<Handle<Prefab<ScenePrefabData>>>,
    animation_index: usize,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct ScenePrefabData {
    transform: Option<Transform>,
    gltf: Option<AssetPrefab<GltfSceneAsset, GltfSceneFormat>>,
    camera: Option<CameraPrefab>,
    light: Option<LightPrefab>,
    tag: Option<Tag<AnimationMarker>>,
}

impl<'a> PrefabData<'a> for ScenePrefabData {
    type SystemData = (
        <Option<Transform> as PrefabData<'a>>::SystemData,
        <Option<AssetPrefab<GltfSceneAsset, GltfSceneFormat>> as PrefabData<'a>>::SystemData,
        <Option<CameraPrefab> as PrefabData<'a>>::SystemData,
        <Option<LightPrefab> as PrefabData<'a>>::SystemData,
        <Option<Tag<AnimationMarker>> as PrefabData<'a>>::SystemData,
    );
    type Result = ();

    fn load_prefab(
        &self,
        entity: Entity,
        system_data: &mut Self::SystemData,
        entities: &[Entity],
    ) -> Result<(), Error> {
        self.transform
            .load_prefab(entity, &mut system_data.0, entities)?;
        self.gltf.load_prefab(entity, &mut system_data.1, entities)?;
        self.camera
            .load_prefab(entity, &mut system_data.2, entities)?;
        self.light
            .load_prefab(entity, &mut system_data.3, entities)?;
        self.tag.load_prefab(entity, &mut system_data.4, entities)?;
        Ok(())
    }

    fn trigger_sub_loading(
        &mut self,
        progress: &mut ProgressCounter,
        system_data: &mut Self::SystemData,
    ) -> Result<bool, Error> {
        self.gltf.trigger_sub_loading(progress, &mut system_data.1)
    }
}

impl<'a, 'b> State<GameData<'a, 'b>> for Example {
    fn on_start(&mut self, data: StateData<GameData>) {
        let StateData { world, .. } = data;

        self.progress = Some(ProgressCounter::default());

        world.exec(
            |(loader, mut scene): (PrefabLoader<ScenePrefabData>, Write<Scene>)| {
                scene.handle = Some(loader.load(
                    "prefab/puffy_scene.ron",
                    RonFormat,
                    (),
                    self.progress.as_mut().unwrap(),
                ));
            },
        );
    }

    fn handle_event(&mut self, data: StateData<GameData>, event: Event) -> Trans<GameData<'a, 'b>> {
        let StateData { world, .. } = data;
        if is_close_requested(&event) || is_key(&event, VirtualKeyCode::Escape) {
            Trans::Quit
        } else if is_key(&event, VirtualKeyCode::Space) {
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
    }

    fn update(&mut self, data: StateData<GameData>) -> Trans<GameData<'a, 'b>> {
        if !self.initialised {
            let remove = match self.progress.as_ref().map(|p| p.complete()) {
                None | Some(Completion::Loading) => false,

                Some(Completion::Complete) => {
                    let scene_handle = data.world
                        .read_resource::<Scene>()
                        .handle
                        .as_ref()
                        .unwrap()
                        .clone();

                    data.world
                        .create_entity()
                        .with(scene_handle)
                        .with(GlobalTransform::default())
                        .build();

                    true
                }

                Some(Completion::Failed) => return Trans::Quit,
            };
            if remove {
                self.progress = None;
            }
            if self.entity.is_none() {
                if let Some(entity) = data.world
                    .exec(|finder: TagFinder<AnimationMarker>| finder.find())
                {
                    self.entity = Some(entity);
                    self.initialised = true;
                }
            }
        }
        data.data.update(&data.world);
        Trans::None
    }
}

fn toggle_or_cycle_animation(
    entity: Option<Entity>,
    scene: &mut Scene,
    sets: &ReadStorage<AnimationSet<usize, Transform>>,
    controls: &mut WriteStorage<AnimationControlSet<usize, Transform>>,
) {
    if let Some((entity, Some(animations))) = entity.map(|entity| (entity, sets.get(entity))) {
        if animations.animations.len() > scene.animation_index {
            let animation = animations.animations.get(&scene.animation_index).unwrap();
            let mut set = get_animation_set::<usize, Transform>(controls, entity);
            if set.has_animation(scene.animation_index) {
                set.toggle(scene.animation_index);
            } else {
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

fn run() -> Result<(), amethyst::Error> {
    let path = format!(
        "{}/examples/gltf/resources/display_config.ron",
        env!("CARGO_MANIFEST_DIR")
    );

    let resources_directory = format!("{}/examples/assets/", env!("CARGO_MANIFEST_DIR"));
    let config = DisplayConfig::load(&path);

    let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([0.0, 0.0, 0.0, 1.0], 1.0)
            .with_pass(DrawPbmSeparate::new().with_transparency(
                ColorMask::all(),
                ALPHA,
                Some(DepthMode::LessEqualWrite),
            )),
    );

    let game_data = GameDataBuilder::default()
        .with(
            PrefabLoaderSystem::<ScenePrefabData>::default(),
            "scene_loader",
            &[],
        )
        .with(
            GltfSceneLoaderSystem::default(),
            "gltf_loader",
            &["scene_loader"],
        )
        .with_bundle(
            AnimationBundle::<usize, Transform>::new("animation_control", "sampler_interpolation")
                .with_dep(&["gltf_loader"]),
        )?
        .with_bundle(
            TransformBundle::new().with_dep(&["animation_control", "sampler_interpolation"]),
        )?
        .with_bundle(VertexSkinningBundle::new().with_dep(&[
            "transform_system",
            "animation_control",
            "sampler_interpolation",
        ]))?
        .with_bundle(
            RenderBundle::new(pipe, Some(config)).with_visibility_sorting(&["transform_system"]),
        )?;

    let mut game = Application::build(resources_directory, Example::default())?.build(game_data)?;
    game.run();
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        error!("Failed to execute example: {}", e);
        ::std::process::exit(1);
    }
}
