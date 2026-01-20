#version 460 core

layout (location = 0) in uint mesh_id;

const uint MAX_VERTEX_DEFINITIONS = 32;

struct Metadata {
    uint offset;
    uint length;
}

struct Vertex {
    float position[3];
    float normal[3];
}

layout(std430, binding = 1) readonly buffer mesh_storage
{
    Metadata metadata[MAX_VERTEX_DEFINITIONS];
    Vertex vertex_storage[];
};

layout(std430, binding = 2) readonly buffer instance_data
{
    mat4 transforms[];
};

uniform mat4 u_projection;
uniform mat4 u_view;

out vec3 fs_world;
out vec3 fs_normal;

void main() {
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

    mat4 transform = transforms[gl_InstanceID];
    vec4 world = transform * vec4(position, 1.0);

    fs_world = world.xyz;
    fs_normal = normal;
    
 E   gl_Position = u_projection * u_view * world;
}