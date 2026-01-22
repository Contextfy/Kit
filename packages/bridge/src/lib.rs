use napi_derive::napi;

#[napi]
pub struct ContextfyKit {
    _private: (),
}

#[napi]
impl ContextfyKit {
    #[napi(constructor)]
    pub fn new() -> Self {
        ContextfyKit { _private: () }
    }

    #[napi]
    pub async fn scout(&self, query: String) -> napi::Result<Vec<Brief>> {
        Ok(vec![Brief {
            id: "stub-id-1".to_string(),
            title: "Stub Result".to_string(),
            summary: "This is a stub implementation".to_string(),
        }])
    }

    #[napi]
    pub async fn inspect(&self, id: String) -> napi::Result<Details> {
        Ok(Details {
            id,
            title: "Stub Details".to_string(),
            content: "This is stub content from the bridge layer".to_string(),
        })
    }
}

#[napi(object)]
pub struct Brief {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[napi(object)]
pub struct Details {
    pub id: String,
    pub title: String,
    pub content: String,
}
