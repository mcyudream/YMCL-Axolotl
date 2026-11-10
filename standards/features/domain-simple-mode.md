# 域傻瓜模式

域管理员在 YMCL 适配器的「傻瓜模式」页面统一配置启用状态、解锁热键、隐藏功能范围、界面入口和域导航项。通过 `GET/PUT /v1/admin/simple-mode` 读取与保存，两个接口均要求 `plugin:ymcl-adapter:design`；保存后广播 `manifest.updated`，配置随 `manifest.simple_mode` 下发。

```json
{
	"schemaVersion": 1,
	"enabled": true,
	"unlockKey": "F12",
	"hiddenScopes": ["domain-pages", "discover", "library", "skins", "worlds", "screenshots", "lab", "downloads", "settings", "create-instance"],
	"hiddenEntrances": ["quick-instances", "create-instance", "settings", "account-sidebar", "domain-switcher", "home-customization"],
	"hiddenNavigationIds": []
}
```

未配置或 `enabled=false` 时不改变现有界面。开启后的默认值如上；域导航栏默认保留，具体条目根据隐藏范围过滤。空数组表示不隐藏该类内容。热键可选 F1 至 F12，缺省为 F12，不带修饰键。

隐藏功能范围会过滤导航并限制对应路由（包括子路由、实例内的世界与截图页面）；隐藏目录会同时隐藏所有子项及对应页面。首页中指向这些页面的域快捷卡片和数据卡片也隐藏，底层首页配置不变。首页及游戏启动始终保留。隐藏入口仅收起该界面入口，不单独限制功能路由。

manifest 的 `simple_mode.hiddenRoutes` 由服务端根据完整导航树即时解析，不接受管理端直接编辑。即使目录或原生导航项已停用、被权限裁剪或不在下发导航中，其隐藏规则仍保留；启动器还会联动隐藏域页面的数据源别名。

热键只执行解锁：重复按键不重新锁定，也不触发 F12 开发工具。解锁状态按域仅保存在启动器进程内存中，本次启动切换到其他域再返回、进入轻量模式后恢复窗口时仍有效；重新启动后恢复傻瓜模式。个人域不受此配置影响，域配置刷新不会清除已经解锁的状态。

此模式控制启动器界面的简化展示，解锁后恢复域原本提供的功能与入口，服务端权限和认证规则继续生效。
