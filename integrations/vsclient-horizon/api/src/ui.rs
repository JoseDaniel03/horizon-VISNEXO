//! Lightweight native controls screen (no external GUI framework or Java mod).
//! Renders on Horizon's screen hook, and only consumes input over its own UI.
use super::controls::ControlsSession;
use horizon_client::mod_api::{Host, ScreenRenderPostEvent, NativeScreenView};

pub const ID:&str="vsclient";
const MAX_ROWS: usize=8;
fn per_page(b:[f32;4])->usize {(((b[3]-88.0)/24.0).floor() as usize).clamp(1,MAX_ROWS)}
fn layout(screen: &NativeScreenView<'_>) -> [f32;4] {
    let [x,y,w,h]=screen.rect;
    let wide=(w-16.0).clamp(180.0,600.0);
    [x+(w-wide)*0.5,y+12.0,wide,h-24.0]
}
fn within(mouse:[f32;2], rect:[f32;4])->bool{
    mouse[0]>=rect[0] && mouse[1]>=rect[1] &&
    mouse[0]<rect[0]+rect[2] && mouse[1]<rect[1]+rect[3]
}
fn row(layout:[f32;4], index:usize)->[f32;4]{
    [layout[0]+10.0,layout[1]+55.0+(index as f32)*24.0,layout[2]-20.0,22.0]
}
pub enum Click {None,Handled,Save}
pub fn click(screen:&NativeScreenView<'_>, controls:&mut ControlsSession, button:i32)->Click {
    if button !=0 {return Click::None;}
    let b=layout(screen);
    let mouse=screen.mouse;
    if !within(mouse,b){return Click::None;}
    if let Some(profile)=controls.current_profile(){
        let page=controls.ui_page();
        let per_page=per_page(b);
        let action_id=profile.actions.iter().skip(page*per_page).take(per_page)
            .enumerate().find(|(i,_)|within(mouse,row(b,*i)))
            .map(|(_,a)|a.id.clone());
        if let Some(id)=action_id {
            controls.begin_capture(&id);
            return Click::Handled;
        }
    }
    let footer=b[1]+b[3]-30.0;
    if mouse[1]>=footer {
        if mouse[0]<b[0]+b[2]*0.33 { controls.page_step(-1,per_page(b));return Click::Handled; }
        if mouse[0]<b[0]+b[2]*0.66 { controls.page_step(1,per_page(b));return Click::Handled; }
        controls.clear_profile_overrides();
        return Click::Save;
    }
    Click::Handled
}

pub fn draw(host:&dyn Host,event:&mut ScreenRenderPostEvent<'_>,controls:&ControlsSession){
    let b=layout(event.screen);
    if b[3]<135.0{return;}
    host.fill(event.batch,b,0xe5101621);
    host.fill(event.batch,[b[0],b[1],b[2],3.0],0xff8b6dbb);
    host.text(event.batch,"VSClient | Controles",[b[0]+10.0,b[1]+9.0],0xffe9ddff,true);
    host.text(event.batch,"Toca una accion para reasignar; Supr para quitar",[b[0]+10.0,b[1]+27.0],0xffbcc1d5,false);
    if let Some(profile)=controls.current_profile(){
        let title=format!("{} / {}",profile.display_name,profile.state);
        host.text(event.batch,&title,[b[0]+10.0,b[1]+42.0],0xffc9b5e9,false);
        let size=per_page(b);
        let skip=controls.ui_page()*size;
        for (i,action) in profile.actions.iter().skip(skip).take(size).enumerate(){
            let r=row(b,i);
            if r[1]+r[3]>b[1]+b[3]-34.0 {break;}
            let bg=if i%2==0 {0xff262a39} else {0xff1d2130};
            host.fill(event.batch,r,bg);
            let binding=controls.effective_binding(&profile.id,action);
            let bound=binding.to_wire();
            let capture=controls.capture_id()==Some(action.id.as_str());
            let right=if capture {"[PULSA TECLA]"} else {bound.as_str()};
            let width=host.text_width(right);
            let limit=(r[2]-width-30.0).max(50.0);
            let mut label=action.display_name.clone();
            while label.len()>3 && host.text_width(&label)>limit {label.pop();}
            host.text(event.batch,&label,[r[0]+5.0,r[1]+6.0],0xfff7f7ff,false);
            host.text(event.batch,right,[r[0]+r[2]-width-8.0,r[1]+6.0],
                if capture {0xffffd073}else{0xff9cc9ff},false);
        }
        let pages=profile.actions.len().div_ceil(size).max(1);
        let counter=format!("Pagina {} / {}",controls.ui_page()+1,pages);
        host.text(event.batch,&counter,[b[0]+b[2]*0.32,b[1]+b[3]-22.0],0xffc7d5e5,false);
    }else{
        host.text(event.batch,"Esperando perfil del servidor...",[b[0]+10.0,b[1]+64.0],0xffedc5b0,false);
    }
    let y=b[1]+b[3]-30.0;
    host.fill(event.batch,[b[0]+8.0,y,b[2]*0.25,25.0],0xff32364b);
    host.fill(event.batch,[b[0]+b[2]*0.68,y,b[2]*0.30,25.0],0xff463b4c);
    host.text(event.batch,"< Anterior",[b[0]+15.0,y+8.0],0xffffffff,false);
    host.text(event.batch,">",[b[0]+b[2]*0.60,y+8.0],0xffffffff,false);
    host.text(event.batch,"Restaurar",[b[0]+b[2]*0.70,y+8.0],0xffffffff,false);
}
