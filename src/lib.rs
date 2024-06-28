use bevy::ecs::system::SystemParam;
use bevy::gltf::Gltf;
use bevy::gltf::GltfMesh;
use bevy::prelude::*;
use bevy::asset::{LoadedFolder, RecursiveDependencyLoadState};
use bevy::render::primitives::Aabb;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use rand::Rng;
use bevy_common_assets::json::JsonAssetPlugin;

pub struct PGModelsPlugin;

impl Plugin for PGModelsPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_plugins(JsonAssetPlugin::<ModelData>::new(&["model.json"]))
        .init_state::<ModelsState>()
        .insert_resource(Models::new())
        .add_systems(Startup, init)
        .add_systems(Update,  track.run_if(in_state(ModelsState::Init)))
        .add_systems(OnEnter(ModelsState::Loaded), process_models)
      ;
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum ModelsState {
    #[default]
    Init,
    Loaded,
    Ready
}


fn init(
    mut commands: Commands, 
    ass:          Res<AssetServer>
){

    let handle_folder_models:     Handle<LoadedFolder> = ass.load_folder("data/models");
    let handle_folder_gltfs:      Handle<LoadedFolder> = ass.load_folder("gltf/");

    commands.insert_resource(
        LoadedDataHandles {
            handle_folder_models,
            handle_folder_gltfs
        });
}

#[derive(Resource)]
struct AssetsCount {
    models: bool,
    gltfs:  bool
}
impl AssetsCount {
    fn ready(&self) -> bool {
        self.gltfs && self.models
    }
}
impl Default for AssetsCount {
    fn default() -> Self {
        AssetsCount{gltfs: false, models: false}
    }
}



fn track(
    mut assets_counts:     Local<AssetsCount>,
    loaded_data:           Res<LoadedDataHandles>,
    ass:                   Res<AssetServer>,
    mut next_model_state:  ResMut<NextState<ModelsState>>
){

    if !assets_counts.models {
        if let Some(data_load_state) = ass.get_recursive_dependency_load_state(&loaded_data.handle_folder_models) {
            if data_load_state == RecursiveDependencyLoadState::Loaded {
                assets_counts.models = true;
            }
        }
    }

    if !assets_counts.gltfs {
        if let Some(gltf_load_state) = ass.get_recursive_dependency_load_state(&loaded_data.handle_folder_gltfs) {
            if gltf_load_state == RecursiveDependencyLoadState::Loaded {
                assets_counts.gltfs = true;
            }
        }
    }

    if assets_counts.ready() {
        next_model_state.set(ModelsState::Loaded)
    }
}





/// Custom system parameter for all things model spawning related so I dont have to copy it everywhere.
#[derive(SystemParam)]
pub struct GLTFS<'w> {
    pub assets_gltf:        Res<'w, Assets<Gltf>>,
    pub assets_gltfmesh:    Res<'w, Assets<GltfMesh>>,
    pub assets_mesh:        ResMut<'w, Assets<Mesh>>
}

#[derive(Resource)]
struct LoadedDataHandles {
    handle_folder_models:         Handle<LoadedFolder>,
    handle_folder_gltfs:          Handle<LoadedFolder>
}


#[derive(Serialize, Deserialize, Clone, Debug, bevy::asset::Asset, bevy::reflect::TypePath, Eq, PartialEq, Hash, Copy)]
pub enum MLib {
    BlueCar,
    RedHouse
}

fn process_models(
    mut commands:          Commands,
    ass:                   Res<AssetServer>,
    mut models:            ResMut<Models>,
    ass_models:            Res<Assets<ModelData>>,
    gltfs:                 GLTFS,
    mut next_model_state:  ResMut<NextState<ModelsState>>
){

    let mut assets_mesh_map: HashMap<String, Handle<Gltf>> = HashMap::new();
    for (gltf_id, _gltf) in gltfs.assets_gltf.iter(){
        let gltf_handle = ass.get_id_handle(gltf_id).unwrap();
        // :D
        info!("{}", gltf_id);
        let mesh_name = ass.get_path(gltf_id)
                           .unwrap()
                           .to_string()
                           .split("/")
                           .last()
                           .unwrap()
                           .to_string()
                           .replace(".gltf", "")
                           .replace(".glb","");
        assets_mesh_map.insert(mesh_name, gltf_handle);
    }

    for (_model_id, model_data) in ass_models.iter(){

        let mut model = Model::new(model_data.clone());
        if let Some(options) = &model_data.options {
            for o in options.iter() {
                let opt_model_name: String = format!("{}{}", o, model_data.model_path);
                let mesh = assets_mesh_map.get(&model_data.model_path).cloned();
                model.meshes.insert(opt_model_name, mesh);
            }
        } else {
            // no options, only one mesh
            let mesh = assets_mesh_map.get(&model_data.model_path).cloned();
            if let Some(_mesh) = mesh {
                model.meshes.insert(model_data.model_path.clone(), Some(_mesh));
            } else {
                model.meshes.insert(model_data.model_path.clone(), None);
            }                
        }
        models.data.insert(model.md.mlib, model);
    }

    next_model_state.set(ModelsState::Ready);
    commands.remove_resource::<LoadedDataHandles>();
}


pub struct GltfData {
    pub mesh:   Handle<Mesh>,
    pub mat:    Handle<StandardMaterial>,
    pub aabb:   Aabb,
}


// Button config to be read from data/gui.json
#[derive(Serialize, Deserialize, Clone, Debug, bevy::asset::Asset, bevy::reflect::TypePath)]
pub struct ModelData {
    pub mlib:            MLib,
    pub model_path:      String,
    pub label:           Option<String>,
    pub height:          Option<f32>,        
    pub scale:           Option<Vec3>,
    pub options:         Option<Vec<String>>, // color options for mesh (for cars)
}

impl ModelData {
    pub fn get_random_option(&self) ->  Option<&str> {

        if let Some(options) = &self.options {
            let mut rng = rand::thread_rng();
            let rand_index = rng.gen_range(0..options.len());
            return Some(&options[rand_index]);

        } else {
            return None;
        }
    }

}

#[derive(Clone, Debug)]
pub struct Model {
    pub md:     ModelData,
    pub meshes: HashMap<String, Option<Handle<Gltf>>>,
}
impl Model {
    pub fn new(md: ModelData) -> Self {
        Model{md, meshes: HashMap::new()}
    }
}


#[derive(Default, Resource, Debug)]
pub struct Models {
    pub data: HashMap<MLib, Model>,
}

impl Models {
    pub fn spawn(
        &self, 
        commands: &mut Commands,
        mlib:     &MLib, 
        gltfs:    &mut GLTFS,
    ){

        if let Some(gltf_data) = self.extract_gltf(mlib, None, gltfs){
            let ent: Entity = commands.spawn((
                PbrBundle {
                    mesh:       gltf_data.mesh,
                    material:   gltf_data.mat,
                    transform: Transform {
                        translation: Vec3::splat(0.0),
                        scale:       Vec3::splat(0.5),
                        ..default()
                    },
                    ..default()
                },
            )).id();
        }
    }


    pub fn new() -> Self {
        Models{data: HashMap::default()}
    }

    pub fn is_valid(&self, mlib: &MLib) -> bool {
        self.data.get(mlib).is_some()
    }

    pub fn get_data(&self, mlib: &MLib) -> Option<&ModelData> {
        if let Some(model) = self.data.get(mlib) {
            return Some(&model.md);
        } else {
            return None;
        }
    }

    pub fn get_mesh(&self, mlib: &MLib, model_path: &str, option: Option<&str>) -> Option<Handle<Gltf>> {
        if let Some(model) = self.data.get(mlib) {
            if let Some(_option) = option {
                return model
                    .meshes
                    .get(&format!("{}{}", _option, model_path))
                    .expect(&format!(
                        "Expected mesh for model path {}{:?}",
                        _option, model_path
                    ))
                    .clone();
            } else {
                return model
                    .meshes
                    .get(model_path)
                    .expect(&format!("Expected mesh for model path {:?}", model_path))
                    .clone();
            }
        } else {
            return None;
        }
    }

    // Extract mesh, material, aabb from gltf
    pub fn extract_gltf(
        &self,
        mlib:               &MLib,
        option:             Option<&str>,
        gltfs:              &GLTFS
    ) -> Option<GltfData> {

        let md = self.get_data(mlib).unwrap();

        if let Some(mesh) = self.get_mesh(mlib, &md.model_path, option) {
            if let Some(gltf) = gltfs.assets_gltf.get(&mesh) {
                let gltf_mesh: &GltfMesh;

                if let Some(_option) = option {
                    let mesh_name = format!("{}{}", _option, md.model_path).into_boxed_str();
                    gltf_mesh = gltfs.assets_gltfmesh.get(gltf.named_meshes
                        .get(&mesh_name)
                        .expect(&format!(" gltf named_meshes expected optional {}, but didnt find it", mesh_name)))
                        .expect(&format!(" assets_gltfmesh expected optional {}, but didnt find it", mesh_name));
                } else {
                    let mesh_name = md.model_path.clone().into_boxed_str();
                    gltf_mesh = gltfs.assets_gltfmesh.get(&gltf.named_meshes[&mesh_name])
                                                    .expect(&format!(" assets_gltfmesh expected {}, but didnt find it", md.model_path));
                }
                let mesh: Handle<Mesh> = gltf_mesh.primitives[0].mesh.clone();
                let mat: Handle<StandardMaterial> = gltf_mesh.primitives[0].material.clone().unwrap();
                let aabb: Aabb = gltfs.assets_mesh.get(&mesh).unwrap().compute_aabb().unwrap();

                return Some(GltfData { mesh, mat, aabb });
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
}

// Calculates mesh scale from aabb
pub fn get_scale(aabb: &Aabb, dims_wh: &(f32, f32)) -> Vec3 {
    let bb_x = aabb.half_extents[0] * 2.0;
    let bb_z = aabb.half_extents[2] * 2.0;
    let scale_x = dims_wh.0 / bb_x;
    let scale_z = dims_wh.1 / bb_z;
    return Vec3::new(scale_x, 1.0, scale_z);
}

