use assets_manager::{AnyCache, Compound};

struct Person {
    name: String,
    avatar: ggez::graphics::Image,
}

struct PersonStruct {
    name: String,
    avatar: String,
}

impl Compound for PersonStruct {
    fn load(
        cache: AnyCache<'_>,
        id: &assets_manager::SharedString,
    ) -> Result<Self, assets_manager::BoxedError> {
        todo!()
    }
}

struct PersonRaw {
    name: String,
    avatar: Storage<ggez::graphics::Image>,
}

impl Compound for PersonRaw {
    fn load(
        cache: AnyCache<'_>,
        id: &assets_manager::SharedString,
    ) -> Result<Self, assets_manager::BoxedError> {
        let PersonStruct { name, avatar } = cache.load_owned(id)?;
        Ok(PersonRaw {
            name,
            avatar: cache.load_owned(&avatar)?,
        })
    }
}

impl GgezAsset for Person {
    type Raw = PersonRaw;

    fn from_raw(raw: PersonRaw, ctx: &mut ggez::Context) -> ggez::GameResult<Self> {
        let PersonRaw { name, avatar } = raw;

        Ok(Person {
            name,
            avatar: avatar.get_or_init(ctx)?.clone(),
        })
    }
}

fn main() {}
