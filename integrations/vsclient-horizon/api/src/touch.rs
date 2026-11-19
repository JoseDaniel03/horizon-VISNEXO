//! A compact single-pointer virtual control strip for Android.
//! No Android SDK, fabricated multitouch or shared launcher state.
//! The platform maps touch to Horizon pointer events; the mod only routes
//! clicks that actually arrive and never steals touch outside its buttons.
pub const MAX_ROWS:usize=4;

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct Slot {pub index:usize,pub rect:[f32;4]}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Hit {Action(usize),Page,None}
fn rect_contains(rect:[f32;4],cursor:[f32;2])->bool{
    cursor[0]>=rect[0] && cursor[1]>=rect[1] &&
    cursor[0]<rect[0]+rect[2] && cursor[1]<rect[1]+rect[3]
}
fn dimensions(gui:[f32;2])->Option<(f32,f32,f32,f32,f32)> {
    let [w,h]=gui;
    if !w.is_finite() || !h.is_finite() || w<100.0 || h<130.0 {return None;}
    let button_w=(w*0.28).clamp(55.0,104.0);
    let button_h=(h*0.09).clamp(25.0,42.0);
    let top=(h*0.31).max(30.0);
    let left=w-button_w-10.0;
    Some((left,top,button_w,button_h,h))
}
pub fn page_size(gui:[f32;2])->usize {
    let Some((_,top,_,button_h,height))=dimensions(gui) else{return 0};
    let space=(height-top-52.0).max(0.0);
    ((space/(button_h+5.0)).floor() as usize).clamp(1,MAX_ROWS)
}
pub fn page_count(total:usize,gui:[f32;2])->usize {
    let size=page_size(gui);
    if size==0 || total==0 {return 0;}
    total.div_ceil(size)
}
pub fn slots(total:usize,page:usize,gui:[f32;2])->Vec<Slot>{
    let Some((left,top,width,height,_))=dimensions(gui) else{return vec![]};
    let size=page_size(gui);
    if size==0{return vec![];}
    let count=page_count(total,gui);
    let current=if count>0 {page%count} else{0};
    (current*size..(current*size+size).min(total)).enumerate().map(|(i,index)|
        Slot{index,rect:[left,top+i as f32*(height+5.0),width,height]})
        .collect()
}
pub fn page_rect(total:usize,gui:[f32;2])->Option<[f32;4]>{
    if page_count(total,gui)<2{return None;}
    let (left,top,width,height,_)=dimensions(gui)?;
    let used=page_size(gui);
    Some([left,top+used as f32*(height+5.0),width,25.0])
}
pub fn hit(cursor:[f32;2],total:usize,page:usize,gui:[f32;2])->Hit{
    if !cursor.iter().all(|v|v.is_finite()){return Hit::None;}
    for slot in slots(total,page,gui) {
        if rect_contains(slot.rect,cursor){return Hit::Action(slot.index);}
    }
    if page_rect(total,gui).is_some_and(|r|rect_contains(r,cursor)){return Hit::Page;}
    Hit::None
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn virtual_controls_are_bounded_and_responsive(){
        let gui=[390.0,780.0];
        assert_eq!(page_size(gui),4);
        assert_eq!(slots(10,0,gui).len(),4);
        assert_eq!(slots(10,2,gui).len(),2);
        let first=slots(10,1,gui)[0];
        assert_eq!(first.index,4);
        assert_eq!(hit([first.rect[0]+4.0,first.rect[1]+4.0],10,1,gui),Hit::Action(4));
        let p=page_rect(10,gui).unwrap();
        assert_eq!(hit([p[0]+4.0,p[1]+4.0],10,1,gui),Hit::Page);
        assert_eq!(hit([50.0,50.0],10,1,gui),Hit::None);
    }
    #[test]
    fn rejects_nonfinite_and_degenerate_viewports(){
        assert_eq!(page_size([f32::NAN,800.0]),0);
        assert!(slots(4,0,[99.0,200.0]).is_empty());
        assert_eq!(hit([f32::INFINITY,0.0],4,0,[390.0,780.0]),Hit::None);
    }
}
