#![allow(clippy::missing_panics_doc, clippy::collapsible_if)]
use proc_macro::{Group, Punct, TokenStream, TokenTree};

///Adds a `compile_error` with the defined message, before the provided token stream
fn comp_error(error: &str, item: TokenStream) -> TokenStream {
    let mut o = format!("compile_error!(\"{error}\");")
        .parse::<TokenStream>()
        .unwrap();
    o.extend([item]);
    o
}

///Describes various struct types
enum StructType {
    ///Normal struct
    ///```
    ///struct A {
    /// ...
    ///}
    ///```
    Regular,
    ///Tupple struct
    ///
    ///```
    ///struct A(...);
    ///```
    Tupple,
    ///Empty struct
    ///
    ///```
    ///struct A;
    ///```
    Empty,
}

///Detiemines if the `TokenStream` is a struct, and what type of struct
fn is_struct_declaration(item: &TokenStream) -> Option<StructType> {
    let items = item.clone().into_iter().collect::<Vec<_>>();
    if items.len() < 3 {
        return None;
    }

    if items
        .last()
        .unwrap()
        .span()
        .source_text()
        .unwrap_or_default()
        == ";"
    {
        if items.len() >= 4 {
            if let TokenTree::Group(_) = items[items.len() - 2] {
                if items[items.len() - 4]
                    .span()
                    .source_text()
                    .unwrap_or_default()
                    == "struct"
                {
                    return Some(StructType::Tupple);
                }
                return None;
            }
        }

        if items.len() >= 3 {
            if items[items.len() - 3]
                .span()
                .source_text()
                .unwrap_or_default()
                != "struct"
            {
                return None;
            }
        }

        return Some(StructType::Empty);
    }

    if items[items.len() - 3]
        .span()
        .source_text()
        .unwrap_or_default()
        != "struct"
    {
        return None;
    }

    Some(StructType::Regular)
}

#[proc_macro_attribute]
///Makes the struct an alias of another `Component`, implementing `Deref`, `DerefMut` and passing all the
///component calls to the aliased component
pub fn alias(attr: TokenStream, item: TokenStream) -> TokenStream {
    //Check if attributes are valid
    let attrs = attr.into_iter().collect::<Vec<_>>();

    if attrs.is_empty() {
        return comp_error("No attribute found, one attribute is rquired", item);
    }

    if attrs.len() != 1 {
        return comp_error("Too many attributes, one attribute is required", item);
    }

    let struct_type = is_struct_declaration(&item);
    if struct_type.is_none() {
        return comp_error("No struct declaration found", item);
    }

    if matches!(struct_type.unwrap(), StructType::Tupple) {
        return comp_error("Tupple structs not supported", item);
    };
    //Actual implementation here

    //Add inner of the type of the attribute
    //Implement Deref, DerefMut
    //Implement Componnent pass all calls to the inner

    let base = attrs[0].span().source_text().unwrap_or_default();
    let items = item.into_iter().collect::<Vec<_>>();
    let name = items[items.len() - 2]
        .span()
        .source_text()
        .unwrap_or_default();

    // Define all the needed blocks

    let inner = format!("__inner: {base}").parse::<TokenStream>().unwrap();

    let deref = format!(
        "
    impl std::ops::Deref for {name} {{ 
        type Target = {base}; 

        fn deref(&self) -> &Self::Target {{ 
            &self.__inner 
        }} 
    }}"
    )
    .parse::<TokenStream>()
    .unwrap();

    let deref_mut = format!(
        "
    impl std::ops::DerefMut for {name} {{ 
        fn deref_mut(&mut self) -> &mut Self::Target {{ 
            &mut self.__inner 
        }} 
    }}
    "
    )
    .parse::<TokenStream>()
    .unwrap();

    let component_impl = format!(
        "
    impl lunar_engine::ecs::Component for {name} {{
        fn mew() -> Self
        where
            Self: Sized,
        {{
            Self {{
                __inner: {base}::mew(),
            }}
        }}
        fn update(&mut self) {{
            self.__inner.update();
        }}
        fn awawa(&mut self) {{
            self.__inner.awawa();
        }}
        fn decatification(&mut self) {{
            self.__inner.decatification();
        }}
        fn set_self_reference(&mut self, reference: ecs::SelfReferenceGuard) {{
            self.__inner.set_self_reference(reference);
        }}
        fn as_any(&self) -> &dyn std::any::Any {{
            self as &dyn std::any::Any
        }}
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {{
            self as &mut dyn std::any::Any
        }}
    }}
    "
    )
    .parse::<TokenStream>()
    .unwrap();

    let mut items = items;

    let tmp = if let TokenTree::Group(i) = items.last().unwrap() {
        let mut s = i.stream();
        s.extend([inner]);
        TokenTree::Group(Group::new(proc_macro::Delimiter::Brace, s))
    } else {
        TokenTree::Group(Group::new(proc_macro::Delimiter::Brace, inner))
    };

    *items.last_mut().unwrap() = tmp;

    let mut o = items.into_iter().collect::<TokenStream>();
    o.extend([deref, deref_mut, component_impl]);

    o
}

#[proc_macro_attribute]
///Creates a marker component, that has no function other then to mark an entity for
///`lunar_engine::ecs::World::get_all_entities_with_component<T>`
pub fn marker_component(attr: TokenStream, item: TokenStream) -> TokenStream {
    //Check if attributes are valid
    if !attr.into_iter().next().is_none() {
        return comp_error("Too many attributes", item);
    }

    let struct_type = is_struct_declaration(&item);
    if struct_type.is_none() {
        return comp_error("No struct declaration found", item);
    }

    if matches!(
        struct_type.unwrap(),
        StructType::Tupple | StructType::Regular
    ) {
        return comp_error("A marker must me an empty struct", item);
    };
    //Actual implementation here

    //Implement Componnent
    let mut items = item.into_iter().collect::<Vec<_>>();
    let name = items[items.len() - 2]
        .span()
        .source_text()
        .unwrap_or_default();

    // Define all the needed blocks
    let component_impl = format!(
        "
    impl lunar_engine::ecs::Component for {name} {{
        fn mew() -> Self
        where
            Self: Sized,
        {{
            Self  
        }}
        fn as_any(&self) -> &dyn std::any::Any {{
            self as &dyn std::any::Any
        }}
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {{
            self as &mut dyn std::any::Any
        }}
    }}
    "
    )
    .parse::<TokenStream>()
    .unwrap()
    .into_iter()
    .collect::<Vec<_>>();

    let derive = r"#[derive(Debug)]"
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>();

    let old = items.splice(0.., derive).collect::<Vec<_>>();
    items.extend_from_slice(&old);
    items.extend_from_slice(&component_impl);

    items.into_iter().collect()
}

#[proc_macro_attribute]
///Defines dependencies of a component must be placed inside the `impl Component` block
pub fn dependencies(attr: TokenStream, item: TokenStream) -> TokenStream {
    //Useless verification that it is a comma separated list.... bc ofc

    //Checking syntax
    //It's my proc macro, i can enforce all the arbitrary rules i want :3
    let mut last_char_type = TokenTree::Punct(Punct::new(',', proc_macro::Spacing::Alone));
    let mut types = Vec::new();

    for t in attr.into_iter() {
        match &t {
            TokenTree::Ident(i) => {
                if matches!(last_char_type, TokenTree::Ident(_)) {
                    return comp_error("Type must be followed by a comma", item);
                }
                types.push(i.clone());
                last_char_type = t.clone();
            }
            TokenTree::Punct(p) => {
                if p.as_char() != ',' || matches!(last_char_type, TokenTree::Punct(_)) {
                    return comp_error(&format!("Invalid token {p}"), item);
                }
                last_char_type = t.clone();
            }
            TokenTree::Literal(t) => return comp_error(&format!("Invalid token {t}"), item),
            TokenTree::Group(t) => return comp_error(&format!("Invalid token {t}"), item),
        }
    }
    let top: String =
        "fn check_dependencies(entity: &lunar_engine::ecs::Entity) -> Result<(), &'static str >{"
            .to_string();
    let top_instanced =
        "fn check_dependencies_instanced(&self,entity: &lunar_engine::ecs::Entity) -> Result<(), &'static str >{";
    let mut body = Vec::new();

    //This is such a hack i love it :3
    for t in types {
        body.push(format!(
            "if !entity.has_component::<{t}>(){{return Err(\"{t}\");}}"
        ));
    }
    let end = "Ok(())}\n";

    let body = body.concat();

    (top + &body + end + top_instanced + &body + end)
        .parse::<TokenStream>()
        .unwrap()
        .into_iter()
        .chain(item.clone().into_iter())
        .collect::<TokenStream>()
}
