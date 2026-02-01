package fi.puppycorp.wgui

import android.os.Bundle
import androidx.activity.ComponentActivity
import android.app.Activity
import android.content.Context
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import fi.puppycorp.wgui.renderer.RenderTree
import fi.puppycorp.wgui.renderer.RendererState
import fi.puppycorp.wgui.renderer.WguiClient
import fi.puppycorp.wgui.ui.theme.WguiTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            WguiTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    WguiApp(modifier = Modifier.padding(innerPadding))
                }
            }
        }
    }
}

@Composable
fun WguiApp(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences("wgui_prefs", Context.MODE_PRIVATE) }
    var serverUrl by remember { mutableStateOf(prefs.getString("server_url", "") ?: "") }
    var editingUrl by remember { mutableStateOf(serverUrl) }
    var connectedUrl by remember { mutableStateOf(serverUrl.takeIf { it.isNotBlank() }) }
    val rendererState = remember { RendererState() }

    if (connectedUrl == null) {
        Column(
            modifier = modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Text(
                text = "Connect to WGUI server",
                style = MaterialTheme.typography.titleMedium
            )
            OutlinedTextField(
                value = editingUrl,
                onValueChange = { editingUrl = it },
                label = { Text("Server URL") },
                placeholder = { Text("http://localhost:12345") },
                modifier = Modifier.fillMaxWidth()
            )
            Button(onClick = {
                val next = editingUrl.trim()
                if (next.isNotEmpty()) {
                    serverUrl = next
                    connectedUrl = next
                    prefs.edit().putString("server_url", next).apply()
                }
            }) {
                Text("Connect")
            }
        }
        return
    }

    SideEffect {
        (context as? Activity)?.title = rendererState.title.value
    }

    var client by remember { mutableStateOf<WguiClient?>(null) }

    LaunchedEffect(connectedUrl) {
        val url = connectedUrl ?: return@LaunchedEffect
        val newClient = WguiClient(
            serverUrl = url,
            onMessages = { messages ->
                rendererState.applyMessages(messages)
            }
        )
        client = newClient
        newClient.connect()
    }

    DisposableEffect(connectedUrl) {
        onDispose {
            client?.close()
            client = null
        }
    }

    RenderTree(
        state = rendererState,
        modifier = modifier.fillMaxSize(),
        client = client
    )
}
