use futures::future::{BoxFuture, FutureExt, Shared};
use std::any::{type_name, TypeId};
use std::collections::HashMap;
use std::marker::PhantomPinned;
use std::mem::transmute;
use std::ops::Deref;
use std::raw::TraitObject;
use std::sync::Mutex;
use tokio::runtime::Handle;

pub trait ModuleTrait: Sync + Send {}
pub trait ModuleFactoryTrait: ModuleTrait {
    fn spawn<'mf>(module_factory: &'mf ModuleFactory) -> BoxFuture<'mf, Self>;
}

pub struct ModuleHandle<T>
where
    T: ModuleTrait + 'static,
{
    module_pool: &'static ModulePool,
    module_reference_cache: &'static T,
}
impl<T> ModuleHandle<T>
where
    T: ModuleTrait + 'static,
{
    fn new(
        module_pool: &'static ModulePool,
        module_reference_cache: &'static T,
    ) -> Self {
        Self {
            module_pool,
            module_reference_cache,
        }
    }
}
impl<T> Deref for ModuleHandle<T>
where
    T: ModuleTrait + 'static,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.module_reference_cache
    }
}
impl<T> Clone for ModuleHandle<T>
where
    T: ModuleTrait + 'static,
{
    fn clone(&self) -> Self {
        self.module_pool.inc::<T>()
    }
}
impl<T> Drop for ModuleHandle<T>
where
    T: ModuleTrait + 'static,
{
    fn drop(&mut self) {
        self.module_pool.dec::<T>()
    }
}

enum ModuleState {
    Initializing {
        initialized_future: Shared<BoxFuture<'static, ()>>,
    },
    Initialized {
        reference_count: usize,
        boxed_trait_object: Box<dyn ModuleTrait>,
    },
}

struct ModulePool {
    modules: Mutex<HashMap<TypeId, ModuleState>>,
}
impl ModulePool {
    pub fn default() -> Self {
        Self {
            modules: Mutex::new(HashMap::new()),
        }
    }

    pub fn inc<T>(&self) -> ModuleHandle<T>
    where
        T: ModuleTrait + 'static,
    {
        // Extract TypeId
        let type_id = TypeId::of::<T>();

        // Modules locked scope
        let object = {
            let mut modules = self.modules.lock().unwrap();

            // This method is used from the module internally, so the module entry must exist and must be initialized
            let (reference_count, boxed_trait_object) = match modules.get_mut(&type_id).unwrap() {
                ModuleState::Initializing { .. } => panic!("Calling inc() on initializing module?"),
                ModuleState::Initialized {
                    ref mut reference_count,
                    ref boxed_trait_object,
                } => (reference_count, boxed_trait_object),
            };

            // Increment reference count
            *reference_count += 1;

            // Cast value into T
            let trait_object = unsafe { transmute::<_, TraitObject>(&**boxed_trait_object) };
            unsafe { &*(trait_object.data as *const T) }
        };

        // Return module handle
        ModuleHandle::new(
            // ModulePool always outlives all modules
            unsafe { transmute::<_, &'static Self>(self) },
            object,
        )
    }
    pub fn dec<T>(&self)
    where
        T: ModuleTrait + 'static,
    {
        // Extract TypeId
        let type_id = TypeId::of::<T>();

        // Modules locked scope
        {
            let mut modules = self.modules.lock().unwrap();

            // This method is used from the module internally, so the module entry must exist and must be initialized
            let reference_count = match modules.get_mut(&type_id).unwrap() {
                ModuleState::Initializing { .. } => panic!("Calling dec() on initializing module?"),
                ModuleState::Initialized {
                    ref mut reference_count,
                    ..
                } => reference_count,
            };

            // Decrement reference count
            *reference_count -= 1;

            // If reference count is zero, drop the value
            if *reference_count == 0 {
                log::trace!("Module {:?} - deinitializing", type_name::<T>());
                modules.remove(&type_id).unwrap();
                log::trace!("Module {:?} - deinitialized", type_name::<T>());
            }
        }
    }
}
impl Drop for ModulePool {
    fn drop(&mut self) {
        if self.modules.lock().unwrap().len() != 0 {
            panic!("ModulePool dropped before dropping all items");
        }
    }
}

pub struct ModuleFactory<'r> {
    runtime: &'r Handle,
    module_pool: ModulePool,
    _pin: PhantomPinned,
}
impl<'r> ModuleFactory<'r> {
    pub fn new(runtime: &'r Handle) -> Self {
        Self {
            runtime,
            module_pool: ModulePool::default(),
            _pin: PhantomPinned,
        }
    }

    pub async fn get<T>(&self) -> ModuleHandle<T>
    where
        T: ModuleFactoryTrait + 'static,
    {
        // Extract TypeId
        let type_id = TypeId::of::<T>();

        let initialized_future = {
            let mut modules = self.module_pool.modules.lock().unwrap();
            let module_state = modules.entry(type_id).or_insert_with(|| {
                // This is used internally in spawn_future, because tokio::spawn requires static lifetime
                // This MAY result in race condition if this object is dropped before the runtime and runtime is still processing
                // That's why we make sure no future is in pending state during drop()
                // FIXME: This probably needs some fixing one day
                let spawn_future_self = unsafe { transmute::<_, &'static ModuleFactory>(self) };

                // This async future will be spawned to default runtime
                // It will initialize the module and replace its value in module pool with initialized version
                let spawn_future = async move {
                    // Spawn the module
                    log::trace!("Module {:?} - initializing", type_name::<T>());
                    let module = T::spawn(spawn_future_self).await;
                    log::trace!("Module {:?} - initialized", type_name::<T>());

                    // Replace initializing state with initialized state
                    let previous_value = spawn_future_self
                        .module_pool
                        .modules
                        .lock()
                        .unwrap()
                        .insert(
                            type_id,
                            ModuleState::Initialized {
                                reference_count: 0,
                                boxed_trait_object: Box::new(module) as Box<dyn ModuleTrait>,
                            },
                        );

                    match previous_value {
                        None => panic!("Module was not previously initialized?!"),
                        Some(ModuleState::Initializing { .. }) => (),
                        Some(ModuleState::Initialized { .. }) => {
                            panic!("Module was already initialized?")
                        }
                    };
                };

                // This future will be shared across waiting futures
                let initialized_future = self
                    .runtime
                    .spawn(spawn_future)
                    .map(|spawn_future_result| spawn_future_result.unwrap())
                    .boxed()
                    .shared();

                // Store initializing state
                ModuleState::Initializing { initialized_future }
            });

            // If the module is not ready, return awaitable future
            // If the module is ready, return None, so the waiting client could proceed to fetching it
            match module_state {
                ModuleState::Initializing { initialized_future } => {
                    Some(initialized_future.clone())
                }
                ModuleState::Initialized { .. } => None,
            }
        };

        // If the module was not initialized when entering this function, await initializing
        if let Some(initialized_future) = initialized_future {
            initialized_future.await;
        };

        // Now operation of incrementing should work as expected
        self.module_pool.inc::<T>()
    }
}
