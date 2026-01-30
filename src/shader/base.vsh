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

layout(std430, binding = 10) readonly buffer MeshStorage
{
    Metadata metadata[MAX_VERTEX_DEFINITIONS];
    Vertex vertex_storage[];
};

struct Entity {
    uint mesh_index;
    uint position_id;
    uint rotation_id;

    // Pad to 16 bytes for compatibility
    // While on my system the ssbo alignment is 4, for now we will force 
    // alignment to 16 bytes to ensure compatibility we might handle this 
    // differently later on.
    uint _pad; 
};

layout(std430, binding = 0) readonly buffer EntityIndexMap 
{
    Entity entities[];
};

layout(std430, binding = 1) readonly buffer MeshData 
{
    uint mesh_ids[];
};

layout(std430, binding = 2) readonly buffer IMap_Positions
{
    uint imap_positions[];
};
layout(std430, binding = 3) readonly buffer IMap_Rotations
{
    uint imap_rotations[];
};

layout(std430, binding = 4) readonly buffer POD_Positions
{
    vec4 pod_positions[]; 
};
layout(std430, binding = 5) readonly buffer POD_Rotations
{
    vec4 pod_rotations[];
};

uniform mat4 u_projection;
uniform mat4 u_view;

out vec3 fs_world;
out vec3 fs_normal;

void main() {
    Entity mapping = entities[gl_InstanceID];
    uint mesh_id_index = mapping.mesh_index;
    uint position_index = imap_positions[mapping.position_id];
    uint rotation_index = imap_rotations[mapping.rotation_id];

    uint mesh_id = mesh_ids[mesh_id_index];
    vec3 position = pod_positions[position_index].xyz;
    vec4 rotation = pod_rotations[rotation_index];

    Metadata metadata = metadata[mesh_id];
    uint offset = metadata.offset;
    uint index = offset + gl_VertexID;

    Vertex vertex = vertex_storage[index];
    vec3 local = vec3(
        vertex.position[0],
        vertex.position[1],
        vertex.position[2]
    );
    vec3 normal = vec3(
        vertex.normal[0],
        vertex.normal[1],
        vertex.normal[2]
    );

    //todo build matrix from translation + rotation and apply
    vec4 world = vec4(local, 1.0);

    fs_world = world.xyz;
    fs_normal = normal;
    
    gl_Position = u_projection * u_view * world;
}