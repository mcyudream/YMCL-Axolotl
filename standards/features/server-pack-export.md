# Mod 端别与服务端导出

Mod 列表展示“仅客户端 / 仅服务端 / 双端 / 未知”，并区分服务端可选的 Mod。
来源提示说明判断来自当前整合包标记、本地通用规则、Modrinth、CurseForge 或 JAR。
已有“运行环境”筛选使用同一份有效端别，不另外维护筛选用标签。

规则优先级为：当前实例规则 > 本地通用规则 > 来源信息 > JAR 声明 > 未知。
来源信息优先使用 Modrinth 版本环境，随后使用项目端别、CurseForge 的明确环境标签。
Fabric / Quilt 读取自身环境声明；旧版 Forge 读取 `@Mod` 的显式 `clientSideOnly` /
`serverSideOnly` 声明，要求同一个 JAR 中所有 Mod 的声明一致。
Forge 依赖的 `side` 和 `displayTest` 不作为自身端别证据。
没有明确证据时保留未知，导出默认包含未知 Mod，供用户检查。

手动规则按来源项目与加载器保存，本地无来源文件使用 Mod ID 或文件 SHA-1。
同一个项目升级、改名后继续使用规则；不同加载器的规则隔离。
移除标记只清除所选范围，随后显示继承规则或自动判断结果。
来源检测按文件 SHA-1 缓存，离线快照仍可使用。

手动端别和服务端导出主要面向整合包维护者，不加入面向玩家的首次启动引导。
操作入口位于内容列表的更多菜单、批量操作和导出窗口，窗口内解释规则范围与导出差异。

## 两种导出

- 完整包：包含所有选中的已启用、非纯客户端 Mod。
- 精简包：筛选相同；有匹配实际文件 SHA-1、大小及 HTTPS 地址的来源文件只记录下载信息，
  首次启动下载并校验实际内容的 SHA-256。无可用来源的 Mod 随包附带。

两者包含服务端常用配置、默认配置、数据包、启动与服务端脚本，排除客户端脚本、
启动器信息、世界、日志和玩家数据。配置不能仅凭 Mod 名可靠拆分，管理员仍需检查客户端选项。
导出预览列出缺失文件和被排除的依赖，后端拒绝省略已知必需依赖的选择。
JAR 声明的服务端依赖参与校验；Forge / NeoForge 的仅客户端依赖与可选依赖不被当成服务端必需项。

导出窗口重新读取实例内容修订号，导出前再次验证。归档按实际复制或扫描的字节生成哈希，
拒绝导出过程中变动的 Mod，通过临时文件原子替换目标。目录遍历跳过符号链接。

压缩包自带 `start-server.sh`、`start-server.cmd` 和 `ymcl-server-bootstrap.py`。
运行脚本需要 Python 3.9+、对应 Java 和另行安装的服务端核心。准备器支持
`--prepare-only`，缓存命中时不重复下载，下载失败不覆盖旧文件，保留修改后的配置。
不自动接受 EULA，不安装 Java 或服务端核心。

## 面板更新器预留协议

根目录 `ymcl-server-manifest.json`：

```json
{
	"formatVersion": 1,
	"format": "ymcl-server-pack",
	"environment": "server",
	"name": "Pack",
	"version": "1.0.0",
	"mode": "slim",
	"minecraft": { "version": "1.20.1", "loader": "fabric", "loaderVersion": "0.16.0" },
	"files": [
		{
			"path": "mods/example.jar",
			"size": 123,
			"hashes": { "sha256": "..." },
			"downloads": ["https://..."],
			"modSide": { "side": "both", "server_support": "required", "source": "modrinth" },
			"providerRefs": []
		}
	],
	"update": {
		"protocol": "ymcl-server-pack/v1",
		"deletePolicy": "previous_manifest_only",
		"preservePaths": ["world/**", "world_nether/**", "world_the_end/**", "server.properties", "ops.json", "whitelist.json", "banned-players.json", "banned-ips.json"],
		"endpoint": null
	}
}
```

`files` 描述用户选中的受管理内容。生成的启动脚本、说明文件和清单本身不在此集合内。
`downloads` 为空时文件随包附带。端别只供审阅，安装以文件清单和哈希为准。

后续面板更新器可对比版本清单，下载到暂存区、验证哈希并替换文件；删除只能涉及旧清单
曾管理、且不属于 `preservePaths` 的路径。世界、玩家数据和管理员配置不因清单缺项而删除。
当前仅预留协议，不声明兼容既有 MIP 服务或已实现远程更新、签名验证与服务重启。
