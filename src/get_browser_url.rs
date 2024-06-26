use uiautomation::controls::ControlType;
use uiautomation::core::UICacheRequest;
use uiautomation::types::Handle;
use uiautomation::types::UIProperty;
use uiautomation::variants::Variant;
use uiautomation::{Result, UIAutomation, UIElement, UITreeWalker};
use windows::Win32::Foundation::HWND;

#[derive(Debug)]
pub struct ElementCriteria {
    name: Option<String>,
    classname: Option<String>,
    controltype: Option<ControlType>,
}

// 判断当前系统是否是中文
pub static mut IS_CHINESE: Option<bool> = None;

pub fn get_brower_url(handle: HWND) -> Result<(String)> {
    // 统计一下花费的时间
    // 先判断是否已经获取到当前 chrome 语言
    unsafe {
        if IS_CHINESE.is_none() {
            // 先利用中文获取一次
            let criteria = ElementCriteria {
                name: Some("地址和搜索栏".to_string()),
                classname: None,
                controltype: None,
            };
            let result = get_brower_url_by_criteria(handle, criteria);
            if result.is_ok() {
                IS_CHINESE = Some(true);
                return Ok(result.unwrap());
            } else {
                IS_CHINESE = Some(false);
                return Ok("".to_string());
            }
        } else {
            if IS_CHINESE.unwrap() {
                let criteria = ElementCriteria {
                    name: Some("地址和搜索栏".to_string()),
                    classname: None,
                    controltype: None,
                };
                return get_brower_url_by_criteria(handle, criteria);
            } else {
                let criteria = ElementCriteria {
                    name: Some("Address and search bar".to_string()),
                    classname: None,
                    controltype: None,
                };
                return get_brower_url_by_criteria(handle, criteria);
            }
        }
    }
}

pub fn get_brower_url_by_criteria(handle: HWND, criteria: ElementCriteria) -> Result<(String)> {
    let automation = UIAutomation::new()?;
    let cache_request: UICacheRequest = automation.create_cache_request().unwrap();
    cache_request.add_property(UIProperty::ControlType).unwrap();
    cache_request.add_property(UIProperty::Name).unwrap();
    cache_request.add_property(UIProperty::ClassName).unwrap();
    let root = automation.element_from_handle_build_cache(Handle::from(handle), &cache_request)?;

    let filter = automation
        .create_property_condition(
            UIProperty::ControlType,
            Variant::from(ControlType::Edit as i32),
            None,
        )
        .unwrap();
    cache_request.set_tree_filter(filter.clone()).unwrap();

    let walker = automation.filter_tree_walker(filter).unwrap();

    if let Some(element) = find_element_with_criteria(&walker, &cache_request, &root, &criteria)? {
        let value = element.get_property_value(UIProperty::ValueValue)?;
        Ok(value.get_string()?)
    } else {
        Ok("".to_string())
    }
}

fn find_element_with_criteria(
    walker: &UITreeWalker,
    cache_request: &UICacheRequest,
    element: &UIElement,
    criteria: &ElementCriteria,
) -> Result<Option<UIElement>> {
    // fn traverse(
    //     walker: &UITreeWalker,
    //     cache_request: &UICacheRequest,
    //     element: &UIElement,
    //     criteria: &ElementCriteria,
    // ) -> Result<Option<UIElement>> {
    //     // 统计一下花费时间
    //     let start = std::time::Instant::now();
    //     let res = _traverse(walker, cache_request, element, criteria);
    //     let end = std::time::Instant::now();
    //     println!("Time cost: {:?}", end - start);
    //     res
    // }

    fn traverse(
        walker: &UITreeWalker,
        cache_request: &UICacheRequest,
        element: &UIElement,
        criteria: &ElementCriteria,
    ) -> Result<Option<UIElement>> {
        // Check if the current element matches the criteria
        if element_matches_criteria(element, criteria)? {
            return Ok(Some(element.clone()));
        }

        // Recursively search the child elements
        if let Ok(child) = walker.get_first_child_build_cache(&element, cache_request) {
            if let Some(found) = traverse(walker, cache_request, &child, criteria)? {
                return Ok(Some(found));
            }

            let mut next = child;
            while let Ok(sibling) = walker.get_next_sibling_build_cache(&next, cache_request) {
                if let Some(found) = traverse(walker, cache_request, &sibling, criteria)? {
                    return Ok(Some(found));
                }
                next = sibling;
            }
        }

        Ok(None)
    }

    fn element_matches_criteria(element: &UIElement, criteria: &ElementCriteria) -> Result<bool> {
        if !criteria.name.is_none() {
            if let Some(ref name) = criteria.name {
                if element.get_name()? != name.clone() {
                    return Ok(false);
                }
            }
        }
        if !criteria.classname.is_none() {
            if let Some(ref classname) = criteria.classname {
                if element.get_classname()? != classname.clone() {
                    return Ok(false);
                }
            }
        }
        if !criteria.controltype.is_none() {
            if let Some(ref controltype) = criteria.controltype {
                if element.get_control_type()? != controltype.clone() {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    traverse(&walker, cache_request, element, criteria)
}
