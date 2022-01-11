use amethyst::{assets::lazy_static, renderer::{RenderBase3D, mtl::FullTextureSet, pass::{
            Base3DPassDef, DrawBase3D, DrawBase3DDesc, DrawBase3DTransparent,
            DrawBase3DTransparentDesc,
        }, rendy::{hal::pso::ShaderStageFlags, mesh::{AsVertex, Normal, Position, Tangent, TexCoord, VertexFormat}, shader::{SpirvShader}}, skinning::JointCombined}};

use std::path::PathBuf;

lazy_static::lazy_static! {
    // These uses the precompiled shaders.
    // These can be obtained using glslc.exe in the vulkan sdk.
    static ref VERTEX: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../assets/shaders/pos_norm_tex.vert.spv"),
        ShaderStageFlags::VERTEX,
        "main",
    ).unwrap();

    static ref VERTEX_SKIN: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../assets/shaders/pos_norm_tex_skin.vert.spv"),
        ShaderStageFlags::VERTEX,
        "main",
    ).unwrap();


    static ref FRAGMENT: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../assets/shaders/outline.frag.spv"),
        ShaderStageFlags::FRAGMENT,
        "main",
    ).unwrap();
}

/// Implementation of `Base3DPassDef` for Physically-based (PBR) rendering pass.
#[derive(Debug)]
pub struct CustomPassDef;
impl Base3DPassDef for CustomPassDef {
    const NAME: &'static str = "Custom";
    type TextureSet = FullTextureSet;
    fn vertex_shader() -> &'static SpirvShader {
        &VERTEX
    }
    fn vertex_skinned_shader() -> &'static SpirvShader {
        &VERTEX_SKIN
    }
    fn fragment_shader() -> &'static SpirvShader {
        &FRAGMENT
    }
    fn base_format() -> Vec<VertexFormat> {
        vec![
            Position::vertex(),
            Normal::vertex(),
            //Tangent::vertex(),
            TexCoord::vertex(),
        ]
    }
    fn skinned_format() -> Vec<VertexFormat> {
        vec![
            Position::vertex(),
            Normal::vertex(),
            //Tangent::vertex(),
            TexCoord::vertex(),
            JointCombined::vertex(),
        ]
    }
}

/// Describes a Physically-based (PBR) 3d Pass with lighting
pub type DrawCustomDesc<B> = DrawBase3DDesc<B, CustomPassDef>;
/// Draws a Physically-based (PBR) 3d Pass with lighting
pub type DrawCustom<B> = DrawBase3D<B, CustomPassDef>;
/// Describes a Physically-based (PBR) 3d Pass with lighting and transparency
pub type DrawCustomTransparentDesc<B> = DrawBase3DTransparentDesc<B, CustomPassDef>;
/// Draws a Physically-based (PBR) 3d Pass with lighting and transparency
pub type DrawCustomTransparent<B> = DrawBase3DTransparent<B, CustomPassDef>;

/// A `RenderPlugin` for forward rendering of 3d objects using physically-based shading.
pub type RenderCustom3D = RenderBase3D<CustomPassDef>;