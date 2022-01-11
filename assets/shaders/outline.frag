#version 450

#include "header/math.frag"

#include "header/environment.frag"

layout(set = 1, binding = 0) uniform Material {
    UvOffset uv_offset;
    float alpha_cutoff;
};

layout(set = 1, binding = 1) uniform sampler2D albedo;
layout(set = 1, binding = 2) uniform sampler2D emission;

layout(location = 0) in VertexData {
    vec3 position;
    vec3 normal;
    vec2 tex_coord;
    vec4 color;
} vertex;

layout(location = 0) out vec4 out_color;

// RGB/HSB conversions from Book of Shaders
vec3 rgb2hsb( in vec3 c ){
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz),
                 vec4(c.gb, K.xy),
                 step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r),
                 vec4(c.r, p.yzx),
                 step(p.x, c.r));
    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)),
                d / (q.x + e),
                q.x);
}

vec3 hsb2rgb( in vec3 c ){
    vec3 rgb = clamp(abs(mod(c.x*6.0+vec3(0.0,4.0,2.0),
                             6.0)-3.0)-1.0,
                     0.0,
                     1.0 );
    rgb = rgb*rgb*(3.0-2.0*rgb);
    return vec3(c.z * mix(vec3(1.0), rgb, c.y));
}


vec3 cel_shading ( vec3 color ) {
    vec3 color_hsb = rgb2hsb(color);
    if (color_hsb.z <= 0.5) {
        color_hsb.z = 0.2;
    } else if (color_hsb.z <= 0.55) {
        color_hsb.z = 0.8;
        color_hsb.y = min(color_hsb.y + 0.7, 1.0);
    } else {
        color_hsb.z = 0.8;
    }
    return hsb2rgb(color_hsb);
}


void main() {
    vec2 final_tex_coords   = tex_coords(vertex.tex_coord, uv_offset);
    vec4 albedo_alpha       = texture(albedo, final_tex_coords);
    float alpha             = albedo_alpha.a;
    if(alpha < alpha_cutoff) discard;

    vec3 albedo = albedo_alpha.rgb;
    vec3 emission = texture(emission, final_tex_coords).rgb;

    vec3 lighting = vec3(0.0);
    vec3 normal = normalize(vertex.normal);
    for (uint i = 0u; i < point_light_count; i++) {
        // Calculate diffuse light
        vec3 light_dir = normalize(plight[i].position - vertex.position);
        float diff = max(dot(light_dir, normal), 0.0);
        vec3 diffuse = diff * normalize(plight[i].color);
        // Calculate attenuation
        vec3 dist = plight[i].position - vertex.position;
        float dist2 = dot(dist, dist);
        float attenuation = (plight[i].intensity / dist2);
        lighting += diffuse * attenuation;
    }
    for (uint i = 0u; i < directional_light_count; i++) {
        vec3 dir = dlight[i].direction;
        float diff = max(dot(-dir, normal), 0.0);
        vec3 diffuse = diff * dlight[i].color;
        lighting += diffuse * dlight[i].intensity;
    }
    lighting = cel_shading(lighting);
    lighting += ambient_color;

    vec3 view_direction = normalize(camera_position - vertex.position);

    vec4 outline_color = vec4(0.0, 0.0, 0.0, 1.0);
    float outline_opacity = 0.0;
    vec4 colorModifier = vec4(0.1, 0.2, 0.2, 1.0 - outline_opacity);

    vec4 final_light = vec4(lighting * albedo + emission, alpha);

    vec4 model_color = final_light * vertex.color;

    float dx = 1.0 / 1024.0;
    float dy = 1.0 / 768.0;

    if (dot(normal, view_direction) <= 0.15) {
        out_color = mix(outline_color, model_color, outline_opacity);
        //out_color = vec4(lighting * albedo + emission, alpha) * vertex.color * colorModifier;
    } else {
        out_color = model_color;
        //discard;
    }
