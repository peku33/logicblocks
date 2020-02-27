pub mod fs;
pub mod sqlite;

use parking_lot::ReentrantMutex;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::transmute;
use std::ops::Deref;
use std::raw::TraitObject;

pub trait Module: Sync + Send + 'static {}
pub trait ModuleFactory: Module {
    fn spawn(context: &Context) -> Self;
}

pub struct Handle<T>
where
    T: Module,
{
    module: &'static T,
    context: &'static Context,
}
// TODO: Add Clone
impl<T> Drop for Handle<T>
where
    T: Module,
{
    fn drop(&mut self) {
        self.context.dec::<T>();
    }
}
impl<T> Deref for Handle<T>
where
    T: Module,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.module
    }
}

enum ContextModulesModule {
    Initializing,
    Initialized {
        reference_count: usize,
        trait_object: Box<dyn Module>,
    },
}

type ModulesMap = HashMap<TypeId, ContextModulesModule>;
type ModulesMapRefCell = RefCell<ModulesMap>;

pub struct Context {
    modules: ReentrantMutex<ModulesMapRefCell>,
}
impl Context {
    pub fn new() -> Self {
        Self {
            modules: ReentrantMutex::new(ModulesMapRefCell::new(HashMap::new())),
        }
    }
    pub fn get<T>(&self) -> Handle<T>
    where
        T: ModuleFactory,
    {
        // Get object type key
        let type_id = TypeId::of::<T>();

        // Reentrant lock
        let modules = self.modules.lock();

        // Try extracting currently registered module
        // Borrow context
        let handle = {
            let mut modules = modules.borrow_mut();

            // Get current ContextModulesModule or initialize with Initializing
            // If at this point module was registered with Initializing step, we have a loop in resolution
            let context_modules_module = modules
                .entry(type_id)
                .and_modify(|context_modules_module| {
                    if let ContextModulesModule::Initializing = context_modules_module {
                        panic!("Deadlock found while resolving {:?}", type_id);
                    }
                })
                .or_insert_with(|| ContextModulesModule::Initializing);

            // Initializing - we have just began initialization (this was checked by .and_notify)
            // Initialized - increase reference count and construct, return
            match context_modules_module {
                // The module has just been put into initializing state, it will be completed in second section
                ContextModulesModule::Initializing => None,
                // The module is already initialized, return the handle
                ContextModulesModule::Initialized {
                    reference_count,
                    trait_object,
                } => {
                    // Convert module to 'static
                    // We can assume this is safe, as Context (owner) always outlives all Handles
                    // This is also checked in Drop
                    let module_static = unsafe {
                        &*(transmute::<_, TraitObject>(&**trait_object).data as *const T)
                    };

                    // Build Handle
                    let handle = Handle {
                        module: module_static,
                        context: unsafe { transmute(self) },
                    };

                    // Since we are returning new handle to existing item, increase reference count
                    *reference_count += 1;

                    // Return handle
                    Some(handle)
                }
            }
        };

        // If module was already initialized
        if let Some(handle) = handle {
            return handle;
        }

        // Initialize the module, at this point:
        // - mutex is locked (but reentrant)
        // - RefCell is not borrowed
        let module = Box::new(T::spawn(&self));

        // Register the module as Initialized
        // Borrow context
        #[allow(clippy::let_and_return)]
        let handle = {
            let mut modules = modules.borrow_mut();

            // Convert module to 'static
            // We can assume this is safe, as Context (owner) always outlives all Handles
            // This is also checked in Drop
            let module_static = unsafe { transmute(&*module) };

            // Build Handle
            let handle = Handle {
                module: module_static,
                context: unsafe { transmute(self) },
            };

            // Replace Initializing with Initialized
            // The handle above is the 1st instance, so we set reference_count to 1
            let context_modules_module_previous = modules.insert(
                type_id,
                ContextModulesModule::Initialized {
                    reference_count: 1,
                    trait_object: module,
                },
            );

            // Make sure that previous state was correct
            match context_modules_module_previous {
                Some(ContextModulesModule::Initializing) => (),
                _ => panic!("Duplicated / missing context_modules_module_previous?"),
            };

            handle
        };

        // Return the final handle
        handle
    }

    fn dec<T>(&self)
    where
        T: Module,
    {
        let type_id = TypeId::of::<T>();

        // Reentrant lock
        let modules = self.modules.lock();

        // Extract trait object if it should be dropped (zero refs remaining)
        // Borrow context
        let trait_object = {
            let mut modules = modules.borrow_mut();

            // Check if object should be removed
            if match modules.get_mut(&type_id) {
                Some(ContextModulesModule::Initialized {
                    reference_count, ..
                }) => {
                    *reference_count -= 1;

                    // Return true if remaining reference count is zero
                    *reference_count == 0
                }
                _ => panic!("Calling dec() on missing / initializing?"),
            } {
                // Yes, so remove it, returning object in Some(...)
                Some(match modules.remove(&type_id) {
                    Some(ContextModulesModule::Initialized {
                        reference_count: 0,
                        trait_object,
                    }) => trait_object,
                    _ => panic!("Calling dec() on missing / initializing / non-zero?"),
                })
            } else {
                // No, return None
                None
            }
        };

        // Modules are kept locked here (is reentrant)
        // However borrow is released
        drop(trait_object);
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        // Reentrant lock
        let modules = self.modules.lock();

        // Borrow context
        {
            let modules = modules.borrow_mut();
            if !modules.is_empty() {
                panic!("Not all modules were released before dropping context?")
            }
        }
    }
}
