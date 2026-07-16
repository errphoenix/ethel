# Ethel
[![Crates.io](https://img.shields.io/badge/crates.io-private--internal-blue)]()

**Game engine toolkit built upon my other project, [Janus](https://github.com/errphoenix/janus), providing data-oriented storage, GPU buffers & shaders abstractions, an asset system, and various utilities.**

### See **[Razed](https://github.com/errphoenix/Razed)**

Ethel expands upon the application context management found in **[Janus](https://github.com/errphoenix/janus)**, turning its thin `Context` and thread-split model into a richer, and slightly opinionated, game engine.

At a high level, Ethel provides:
* `Context` state with the `StartupHandler` trait: providing a more in-depth and opinionated initialization process for the application, tightly coupled to the `StateHandler` and `RenderHandler` traits, which represent Janus' `State` and `Render`.
* Full frame lifecycle: `StateHandler` builds upon Janus' `Update` trait, providing more a richer frame lifecycle with the `fixed_step`, `upload_gpu`, `on_new_frame`, `on_key_event` functions. The `Draw` trait is also expanded by `RenderHandler`, with the `pre_frame` and `render_frame` functions.
* Asset system: Ethel expands on Janus' `fnv1a` string hashing, providing the `lazy_hash_str!` macro to lazily hash and cache strings. These are used in Ethel's asset system as `AssetId`'s to look up resources in an `AssetManager<T>`. Each asset entry must implement the `Import` and `Upload` traits. Optionally, an asset may also implement the `HasMetadata<M>` trait, allowing for the use of `AssetMetadataRegistry<M>`, essential to access the `AssetRegistry` (living on the render thread) from the simulation thread and any other.
* OpenGL buffers abstractions: mainly regarding persistent mapped SSBO's, Ethel provides a few types:
    * `TriBuffer<T>`: a triple buffered persistent mapped SSBO over a single type `T`, allowing for safe read and writes to the GPU buffers
    * `PartitionedTriBuffer`: a triple buffer persistent mapped SSBO over multiple types. Each triple-buffer section is divided into "partitions". Reading and writing to the partitions is not safe, but Ethel provides the `layout_buffer!` macro to safely initialize and manage `PartitionedTriBuffer`'s to respect correct partitions lengths and offsets and the SSBO alignment offset supported by the machine.
    * `ImmutableBuffer<T>`: an OpenGL buffer initialized only once, for static, immutable GPU data. Not a triple buffer, and it is only mapped during initialization.
* GPU command queue: an indirect draw command queue built around the `DrawGroups` trait, in order to differentiate commands in the queue and let the render thread dispatch depending on the current "bound" draw group.
* Compile-time shader abstractions: through the `shader_glsl!` macro and various other related macros to define compile-time type-checked GLSL shaders, supporting vertex, pixel (fragment), and compute shaders, allows you to define uniforms, attributes, SSBOs, custom types. Also takes care of uniform location caching.
* Data-oriented storage: once again, Ethel provides several options:
    * `IndexArrayColumn<T>` and `ParallelIndexArrayColumn<T>` for single-type columnar, contiguous storage. The first option keeps the stable `IndirectIndex` contiguous to each `T` element inside the contiguous data vector, the latter instead has *another* vector of `IndirectIndex` parallel to it. This is useful if, during data processing, you often need the `IndirectIndex` of the element, saving a look-up. 
    * `*RowTable` multi-column tables up to 12 columns, defined with the `table_spec!` macro. Functionally identical to `ParallelIndexArrayColumn`.
    
    These DOD, columnar data storage types are built on generational indices. These are:
    * `IndirectIndex` (stable, costs an indirection to look up the `DirectIndex`)      
    * `DirectIndex` (direct index to the contiguous storage, less stable: the `DirectIndex` of an element may change if elements are removed).
* Spatial hash implementations: these are built upon `rustc_hash`, Ethel provides 2 options:
    * `FxSpatialHash<T>` where each spatial `Cell` is mapped to a single value `T`
    * `FxLsSpatialHash<T>` where each spatial `Cell` is mapped to a bucket of values `T`
* Triple-buffered synchronisation: Ethel manages the Janus' simulation thread - render thread (and gpu) synchronisation with a triple buffer. This is already implemented within Ethel, the programmer is only required to define the data to be shared across the boundary.
* Smaller utilities: 
    * Mesh staging system to prepare meshes for OpenGL, either for standard attributes or vertex pulling with SSBOs.
    * The `AccumulationWindow` type for time-based value accumulation, used in Razed for debris sleeping.
    * The `camera` module providing camera utilities and an orbital camera implementation (used in Razed).
    

## Purpose
The purpose of Ethel is to expand upon **[Janus](https://github.com/errphoenix/janus)**' minimal and unopinionated systems into a more feature-rich game engine layer for **[Razed](https://github.com/errphoenix/Razed)**.

Ethel is more opinionated: it defines how state and rendering are to be structured and how assets are managed between threads and the development experience with GPU buffers and shaders.

As with Janus, Ethel has no specific roadmap and expands depending on the needs of Razed.

## Development with Ethel
### Preliminaries
The Ethel layer requires, first and foremost, 2 fundamental types to be defined:
* The shared data for the thread boundary between the simulation thread and render thread (and gpu). There are no trait bounds here. 
    
    An example:
    ```rust
    struct SharedData {
    	pub draw_commands: TriBuffer<DrawCommand>,
    	pub entity_ssbo_data: PartitionedTriBuffer<PARTITIONS_LEN>,
    	// ...whatever other THREAD SAFE type you may need
    	// Janus's TriCell can be used as well
    	// mutexes and rwlocks are discouraged
    }
    ```
* The draw groups to be interpreted by the render thread. This is just an enum (or a unit struct if there are no distinguished groups) implementing the `DrawGroups` trait. 

    An example:
    ```rust
    enum MyDrawGroups {
    	Entity,
    	DebugLines,
    	Hitboxes,
    }
    impl DrawGroups for MyDrawGroups {
    	// these are to used for debugging
    	// this function might become optional in the future
    	fn as_str(&self) -> &'static str {
    		use Self::*;
    	
    		match self {
    		    Entity => "entity",
    		    DebugLines => "dbg_lines",
    		    Hitboxes => "hitboxes",
    		}
    	}
    }
    ```

### Usage of `StateHandler` and `RenderHandler`
These wrap around the minimal `Update` and `Draw` Janus traits.

#### State example:
```rust
#[derive(Default)] // or implement yourself
struct ImState {
	// columns, tables, asset metadata registries...
    ...
}
// handlers must define the frame data and draw groups they work with
impl StateHandler<SharedData, MyDrawGroups> for ImState {
	fn on_new_frame(
		&mut self,
		input: &mut InputSystem, 
		screen: &mut Mirror<ScreenSpace>,
		view_point: &TriCell<ViewPoint>,
		delta: DeltaTime, // time since last new frame
	) {
		// this operation only runs once per-frame: this is important
		// in relation to fixed_step()
	}
	
	fn on_key_event(&mut self, event: KeyEvent) {
		// sequential key/mouse button processing, for text input
		// ideally, these would be buffered and processed at once
	}
	
	fn fixed_step(
		&mut self, 
		input: &mut InputSystem,
        screen: &mut Mirror<ScreenSpace>,
        view_point: &TriCell<camera::ViewPoint>,
        // fixed step duration
        delta: DeltaTime,
	) {
		// fixed step delta-accumulated function, ideal for physics
		// while the function provides access to the input system,
		// this is not recommended: use on_new_frame
	}

	fn upload_gpu(
		&mut self, 
		// the Cross PRODUCER boundary, uploading data to the 
		// gpu or render thread through SharedData
		boundary: &Cross<Producer, SharedData>,
		command_queue: &mut GpuCommandQueue<MyDrawGroups>
	) {
		// you could populate the gpu command queue here
		
		// cross the thread boundary: safely access the triple buffer
		// this is where proper gpu upload occurs
		// this operation CAN BLOCK if the render thread cannot catch up
		// with the simulation thread. This is deliberate: the simulation 
		// thread will never produce frames the render thread cannot use
	    boundary.cross(|
	    	// the triple buffer section we are working on
	    	// used to access triple-buffered types
	    	section,
	    	// shared boundary data
	    	storage,
	    | {
	    	// storage is your SharedData defined in the section above
	    	// this contains GPU buffer abstractions
	    	// upload operations occur right here: ideally, read from 
	    	// contiguous tables/columns and upload to the gpu directly
	    	// though persistent mapping
	    })
	}
}
```

#### Render example:
```rust
#[derive(Default)] // or implement yourself
struct ImRender {
	// shaders, asset registries..
	...
}
impl RenderHandler<SharedData> for ImRender {
	fn init_resources(&mut self, resolution: Resolution) {
		// one-time initialization of resources
		// ideal for shaders, framebuffers...
	}
	
	fn pre_frame(
		&mut self,
		screen: &mut Mirror<ScreenSpace>,
		view: &TriCell<ViewPoint>,
		// time since last rendered frame
		delta: DeltaTime
	) {
		// pre-rendering operations
		// ideal for setting up uniforms, resizing framebuffers..
		// any operation that doesnt depend on shared data
	}
	
	fn render_frame(
		&mut self,
		// shared thread-safe data defined in the first section
		frame_data: &SharedData,
		// section of the triple buffer we are working on
		section: StorageSection,
	) {
		// while StateHandler::upload_gpu must explicitly call cross(),
		// the RenderHandler does so implicitly, allowing ethel to manage
		// gl query objects to ensure triple buffer safety
		// it is ideal to keep this as lightweight as possible 
		// operations that do not require any shared data should be moved
		// to pre_frame
	}
}
```
### Wiring it together: `StartupHandler`
`StartupHandler` wraps around Janus' `Setup` trait, managing a more nuanced state initialization.

The initialization remains largely the same as in Janus:
```rust
fn main() {
	let (input_sys, input_dispatch) = janus::input::stream();
	
	let mut startup_handler = StartupHandler::new(
		input_sys, 
		// initialization of the shared data defined 
		// in the first section
		|| SharedData::new()
	);
	
	// ethel mesh staging for mesh initialization
	// this processed mesh data and setups metadata for access
	// in shaders through vertex pulling
	let mesh_metadata = MeshStaging::new();
	// ..load mesh data
	
	// register mesh data and metadata
	startup_handler.with_mesh_data(mesh_data);
	// set the SSBO layout for mesh data
	// either define your own with `layout_buffer!`, or
	// use Ethel's `layout_mesh_buffer!` to initialize
	// the default layout, with your defined capacities.
	startup_handler.with_mesh_layout(...);
	
	// setup default OpenGL state
	// this is called right after OpenGL context creation
	startup_handler.with_gl_state(|| {
		// ......
	});
	
	// create Janus context
	let ctx = janus::context::Context::new(
		// these are not generics: these are Ethel's internal types
		|state: &mut State, renderer: &mut Renderer| {
			state.handler_init_callback(|inner| {
				// optional callback to modify YOUR inner 
				// state during initialization
				// this allows mutable access to ImState
			});
			
			renderer.handler_init_callback(|inner| {
				// optional callback to modify YOUR inner 
				// renderer during initialization
				// this allows mutable access to ImRender
			});
			
			// finalize startup
			startup_handler.init(state, renderer);
		}, 
		input_dispatch,
		DISPLAY_PARAMS, // defined like in Janus
	);
	
	// run application	
	janus::run(ctx);
}
``` 
### Creating data-oriented multi-columnar storage
This is done with the `table_spec!` macro mentioned before, allowing to create DOD tables of up to 12 columns.

#### Example
```rust
ethel::table_spec! {
    struct Nodes {
        predicted_pos: glam::Vec3;
        current_pos: glam::Vec3;

        mass: f32;
        inv_mass: f32;
        forces: glam::Vec3;
        velocity: glam::Vec3;
    }
}
```

This generates `NodesRowTable`.

### Generating OpenGL SSBO layout abstractions
As mentioned before, this can be done with the `layout_buffer!` macro, and it is essential when working with `PartitionedTriBuffer`.

The macro generates a `#[repr(usize)]` enum to safely index partitions in a `PartitionedTriBuffer` that follows that layout.

#### Example: descriptor
```rust
pub const DEBRIS_ALLOC: usize = 131072;
pub const DEBRIS_STORAGE_PARTS: usize = 3;

layout_buffer! {
    const DebrisData: DEBRIS_STORAGE_PARTS, {
        enum PodPositions: DEBRIS_ALLOC => {
         	// rust type, this also defines the type layout used by OpenGL,
         	// so it is recommended to use #[repr(C)] types or simple type
         	// layouts like arrays of primitives
         	// an array of 4 floats is essentialy a vec4.
         	// keep in mind that a vec3 (3 floats) should not be used due
         	// to OpenGL ssbo alignment requirements
            type [f32; 4];
            // internal binding index, keep sequential (0, 1, 2, 3)..
            bind 0; 
            // OPTIONAL ssbo binding index for this partition
            shader 2;
        };
        enum PodRotations: DEBRIS_ALLOC => {
            type [f32; 4];
            bind 1;
            shader 3;
        };
        enum PodMeshId: DEBRIS_ALLOC => {
            type ethel::mesh::Id;
            bind 2;
            shader 4;
        };
    }
}

```
#### Example: usage
```rust
// INITIALIZATION
let debris_layout = LayoutDebrisData::create();
let debris_buffer = PartitionedTriBuffer::new(debris_layout);
LayoutDebrisData::initialize_partitions(&debris_buffer);

// this should occur during SharedData initialization
// debris_buffer will be stored in our SharedData...

// UPLOAD
let buf_idx = storage_section.as_index(); // section of the triple buffer
let debris_positions: &[glam::Vec3] = ...; 
let debris_rotations: &[glam::Quat] = ...;
let debris_mesh_ids: &[ethel::mesh::Id] = ...;

let debris = &shared_data.debris;

// SAFETY: the use of LayoutDebrisData ensures we blit to a
// valid section of the partitioned buffer.
unsafe {
    debris.blit_part_padded(
        buf_idx,
        LayoutDebrisData::PodPositions as usize,
        debris_positions,
        0, // offset 0
        // since we store vec3's on the cpu to save on memory but
        // OpenGL, due to quirky alignment requirements, does not like
        // an SSBO of vec3's, we pad to a vec4
        4, // PAD 4 BYTES
	);
	debris.blit_part(
        buf_idx,
        LayoutDebrisData::PodRotations as usize,
        debris_rotations,
        0, // offset 0
    );
    debris.blit_part(
        buf_idx,
        LayoutDebrisData::PodMeshId as usize,
        debris_mesh_ids,
        0, // offset 0
    );
}

// BIND SSBOs (inside render_frame)
let buf_idx = storage_section.as_index(); // section of the triple buffer

let debris = &storage.debris;
// bind all partitions of the buf_idx section to the ssbo indices
// defined in its layout descriptor
debris.bind_shader_storage(buf_idx); 

// or bind a single partition to an ssbo, leaving None will bind it to the
// default index defined in its layout descriptor, or we can specify it
debris.bind_shader_storage_single(
	buf_idx,
	LayoutDebrisData::PodPositions as usize,
	Some(5),
);

```

Look at the doc comment of `layout_buffer!` for additional information.

### Writing shaders
As mentioned above, the `shader_glsl!` macro allows you to write compile-time type-checked shaders, and simplifies shader usage and management.

See the macro's doc comment for additional information.

#### Example: descriptor
See [Razed's shaders](https://github.com/errphoenix/Razed/tree/main/Razed/src/render/shaders) for some examples of shader descriptors used.

#### Example: initialization and usage in rendering
```rust
// INITIALIZATION: ideally in init_resources
let shader = ShaderNameShader::new_compiled();
// assign to our renderer
self.shader_name_shader = shader;

// USAGE IN RENDERING
// you would likely want to do this in pre_frame
let shader = &self.shader_name_shader;
shader.bind();
// for a uniform named 'projection' of glsl type mat4
// of length 1 and Rust type [f32; 16]:
let proj_matrix: [f32; 16] = ...get projection
shader.uniform_projection_mat4v([proj_matrix]);

// the bind in render_frame again, when necessary
self.shader_name_shader.bind();

```

## License
Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.