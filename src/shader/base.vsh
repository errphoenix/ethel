#version 460 core

const uint MAX_VERTEX_DEFINITIONS = 64;

struct Metadata {
    uint offset;
    uint length;
};

struct Vertex {
    float position[3];
    float normal[3];
};

layout(std430, binding = 5) readonly buffer mesh_storage
{
    Metadata metadata[MAX_VERTEX_DEFINITIONS];
    Vertex vertex_storage[];
};

struct Entity {
    uint mesh;
    uint transform;
};

layout(std430, binding = 0) readonly buffer EntityMap 
{
    Entity entities[];
};

layout(std430, binding = 1) readonly buffer MeshData 
{
    uint mesh_ids[];
};
layout(std430, binding = 2) readonly buffer Transforms
{
    mat4 transforms[];
};

uniform mat4 u_projection;
uniform mat4 u_view;

out vec3 fs_world;
out vec3 fs_normal;

void main() {
    Entity mapping = entities[gl_InstanceID];
    uint mesh_id_index = mapping.mesh;
    uint transform_index = mapping.transform;

    uint mesh_id = mesh_ids[mesh_id_index];
    mat4 transform = transforms[transform_index];

    Metadata metadata = metadata[mesh_id];
    uint offset = metadata.offset;
    uint index = offset + gl_VertexID;

    Vertex vertex = vertex_storage[index];
    vec3 position = vec3(
        vertex.position[0],
        vertex.position[1],
        vertex.position[2]
    );
    vec3 normal = vec3(
        vertex.normal[0],
        vertex.normal[1],
        vertex.normal[2]
    );

    vec4 world = transform * vec4(position, 1.0);

    fs_world = world.xyz;
    fs_normal = normal;
    
    gl_Position = u_projection * u_view * world;
}